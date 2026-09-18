use crate::models::{SyncConfigDTO, SyncResult};
use crate::services::webdav::{WebdavClient, WebdavFile};
use rusqlite::{params, Connection};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// 可恢复快照的最低 schema 版本：更早的库缺少必需表，且其迁移链未经历史 fixture 验证。
const MIN_RESTORE_SCHEMA_VERSION: i64 = 8;

/// 快照体积上限（2 GiB）。这不是产品配额，而是「这份文件根本不像是本应用的数据库」的兜底判断：
/// 远端被替换成音视频、目录转储或超大文件时立刻拒绝，不进入 open/迁移流程。
const MAX_SNAPSHOT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// 远程快照文件名与其校验和 sidecar 名。
const REMOTE_SNAPSHOT_NAME: &str = "lumo.sqlite";
const REMOTE_CHECKSUM_NAME: &str = "lumo.sqlite.sha256";

/// 恢复前副本（救援副本）的文件名后缀，见 [`SyncService::rescue_backup_path`]。
const RESCUE_BACKUP_SUFFIX: &str = "restore_bak";

/// SQLite 文件头魔数（前 16 字节）。合法数据库必然以它开头。
const SQLITE_HEADER_MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// 读取旧版同步密钥（硬编码，全设备相同）——仅用于读取历史存量数据（P1-08），
/// 读取成功后由 get_config 懒迁移为 v3 机器绑定格式。
fn derive_sync_key() -> [u8; 32] {
    let seed = b"com.hao.lumo.sync";
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = seed[i % seed.len()];
    }
    key
}

/// V3：机器绑定加密（.device_secret 派生 key，P1-08）。
/// DB 单独泄露不再可解密（密钥在密钥文件中，与 DB 不同文件、不同泄露面）。
fn encrypt_sync_password_v3(machine_key: &[u8; 32], password: &str) -> String {
    use base64::Engine;
    let bytes: Vec<u8> = password
        .bytes()
        .enumerate()
        .map(|(i, b)| b ^ machine_key[i % 32])
        .collect();
    format!(
        "v3:seal:{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    )
}

/// 解密同步密码：按前缀分发 v3（机器绑定）/ 旧格式（硬编码 key）。
/// 旧格式解密成功时同步触发懒迁移（UPDATE 为 v3），返回 (密码, 是否发生迁移)。
fn decrypt_sync_password(
    encoded: &str,
    machine_key: &[u8; 32],
    conn: &Connection,
) -> Option<String> {
    use base64::Engine;
    if let Some(payload) = encoded.strip_prefix("v3:seal:") {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .ok()?;
        let decrypted: Vec<u8> = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ machine_key[i % 32])
            .collect();
        return String::from_utf8(decrypted).ok();
    }

    // 旧格式：硬编码 key
    let key = derive_sync_key();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let decrypted: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % 32])
        .collect();
    let password = String::from_utf8(decrypted).ok()?;
    // 懒迁移：升级为 v3 机器绑定格式
    let v3 = encrypt_sync_password_v3(machine_key, &password);
    let _ = conn.execute(
        "UPDATE sync_config SET password_encrypted = ?1 WHERE id = 1",
        params![v3],
    );
    Some(password)
}

/// 同步服务：管理 sync_config 的 CRUD 及 WebDAV 上传/下载/浏览。
pub struct SyncService;

impl SyncService {
    /// 校验并迁移一份下载的快照，使其可以安全进入本机 live 库。
    ///
    /// 顺序至关重要：**先判定「这是不是一份 Lumo 数据库」，再迁移**。
    /// `db::init_db` 对任何可打开的 SQLite 文件都会建表并把版本推到最新，
    /// 所以若先迁移，远端的 0 字节文件 / 其它软件的 DB 也会被加工成
    /// 「看起来合法」的 Lumo 库，随后整库覆盖本地数据。
    pub fn validate_snapshot(path: &Path) -> Result<(), String> {
        let result = Self::validate_and_migrate_snapshot(path);
        // 校验过程会以 WAL 打开快照（见 db::init_db）。-wal 必须与主文件配对才有意义，
        // 因此所有连接释放后再收掉辅助文件：留着它，下一次恢复会拿半截 WAL 去"修复"一个新主文件。
        Self::remove_sidecar_files(path);
        result
    }

    fn validate_and_migrate_snapshot(path: &Path) -> Result<(), String> {
        let version_before = Self::assert_lumo_snapshot(path)?;

        // 历史 schema：先迁移到本机版本，恢复后 live 库无需再次升级即可使用
        crate::db::init_db(path.to_path_buf())
            .map_err(|e| format!("无法初始化同步数据库快照: {}", e))?;

        let conn = Connection::open(path).map_err(|e| format!("无法打开同步数据库快照: {}", e))?;

        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| format!("同步数据库完整性检查失败: {}", e))?;
        if !integrity.eq_ignore_ascii_case("ok") {
            return Err(format!("同步数据库完整性检查未通过: {}", integrity));
        }

        // 外键检查：VACUUM INTO 不校验引用一致性，快照脱敏等写入也可能留下孤儿行，
        // 孤儿会在新版本 UI 里渲染成空白条目，宁可在这里拒绝。
        let orphans: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (SELECT * FROM pragma_foreign_key_check() LIMIT 1)",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if orphans > 0 {
            return Err("同步数据库存在外键不一致的记录，已拒绝恢复".to_string());
        }

        let required_tables = [
            "schema_migrations",
            "sources",
            "tracks",
            "media_files",
            "sync_config",
        ];
        for table in required_tables {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get(0),
                )
                .map_err(|e| format!("检查同步数据库结构失败: {}", e))?;
            if !exists {
                return Err(format!("同步数据库缺少必要表: {}", table));
            }
        }

        let version_after = Self::snapshot_schema_version(&conn)?;
        if version_after < version_before {
            return Err(format!(
                "快照迁移后版本回退（{} < {}），已拒绝恢复",
                version_after, version_before
            ));
        }
        Ok(())
    }

    /// 迁移前的甄别：文件头是 SQLite、能打开、schema 版本落在本机可处理区间。
    /// 返回迁移前版本号。任何一条不满足都说明远端那份东西不该覆盖本地库。
    fn assert_lumo_snapshot(path: &Path) -> Result<i64, String> {
        let size = std::fs::metadata(path)
            .map_err(|e| format!("无法读取下载的文件: {}", e))?
            .len();
        if size < SQLITE_HEADER_MAGIC.len() as u64 || size > MAX_SNAPSHOT_BYTES {
            return Err(format!("云端快照大小异常（{} 字节），已拒绝恢复", size));
        }

        let mut header = [0u8; 16];
        let mut file = File::open(path).map_err(|e| format!("无法打开下载的文件: {}", e))?;
        file.read_exact(&mut header)
            .map_err(|e| format!("读取文件头失败: {}", e))?;
        if &header != SQLITE_HEADER_MAGIC {
            // 典型成因：登录重定向返回的 HTML 错误页、网关提示页、目录列表
            return Err(
                "云端文件不是有效的 SQLite 数据库（服务器可能返回了错误页面），已拒绝恢复"
                    .to_string(),
            );
        }

        let conn = Connection::open(path).map_err(|e| format!("无法打开同步数据库快照: {}", e))?;
        let version = Self::snapshot_schema_version(&conn).map_err(|e| {
            // 读不到版本号 = 没有 schema_migrations 表 = 这不是 Lumo 的库
            format!("云端文件不是 Lumo 数据库，已拒绝恢复（{}）", e)
        })?;
        if version < MIN_RESTORE_SCHEMA_VERSION {
            return Err(format!(
                "同步数据库版本过旧（V{}），需要至少 V{}，请先在来源设备上升级并重新备份",
                version, MIN_RESTORE_SCHEMA_VERSION
            ));
        }
        if version > crate::db::TARGET_SCHEMA_VERSION {
            return Err(format!(
                "云端快照来自更新版本的 Lumo（V{}），当前版本最高支持 V{}，请先升级本机再恢复",
                version,
                crate::db::TARGET_SCHEMA_VERSION
            ));
        }
        Ok(version)
    }

    fn snapshot_schema_version(conn: &Connection) -> Result<i64, String> {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("读取同步数据库版本失败: {}", e))
    }

    /// 删除 SQLite 随文件产生的 -wal / -shm 辅助文件。
    fn remove_sidecar_files(path: &Path) {
        for suffix in ["-wal", "-shm"] {
            let mut p = path.as_os_str().to_os_string();
            p.push(suffix);
            let _ = std::fs::remove_file(PathBuf::from(p));
        }
    }

    /// 清理历史恢复残留的下载临时文件（下载中途进程被杀时会留下）。
    /// 只删 1 小时前的，避免动到同一时刻另一次正在写入的下载。
    fn cleanup_stale_downloads(app_dir: &Path) {
        let Ok(entries) = std::fs::read_dir(app_dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(REMOTE_SNAPSHOT_NAME) || !name.ends_with(".download") {
                continue;
            }
            let stale = entry
                .metadata()
                .and_then(|m| m.modified())
                .map(|t| t.elapsed().map(|d| d.as_secs() > 3600).unwrap_or(false))
                .unwrap_or(false);
            if stale {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    /// 本轮「恢复前副本」的唯一路径（`lumo.sqlite.<pid>-<纳秒>.restore_bak`）。
    ///
    /// 必须每轮唯一：固定名会让第二次恢复在开工第一步就删掉第一次失败时留下的副本，
    /// 而那份副本在回滚也失败的场景里是用户唯一的历史数据（CR-001）。
    pub fn rescue_backup_path(app_dir: &Path) -> PathBuf {
        app_dir.join(format!(
            "{}.{}.{}",
            REMOTE_SNAPSHOT_NAME,
            staging_suffix(),
            RESCUE_BACKUP_SUFFIX
        ))
    }

    /// `app_dir` 下尚未处理的救援副本，按文件名排序（越早的一轮越靠前）。
    ///
    /// 这些文件只能由成功流程或用户明确操作删除，因此这里只负责**发现并告知**，
    /// 不做任何自动清理。老版本（固定名 `lumo.sqlite.restore_bak`）留下的遗留副本
    /// 同样落在这个模式里，升级后也会被一并告知而不是被忽略。
    pub fn list_rescue_backups(app_dir: &Path) -> Vec<PathBuf> {
        let prefix = format!("{}.", REMOTE_SNAPSHOT_NAME);
        let suffix = format!(".{}", RESCUE_BACKUP_SUFFIX);
        let Ok(entries) = std::fs::read_dir(app_dir) else {
            return Vec::new();
        };
        let mut found: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with(&prefix) && name.ends_with(&suffix))
                    .unwrap_or(false)
            })
            .collect();
        found.sort();
        found
    }

    /// 读取同步配置（密码解密后返回）。需要机器密钥（P1-08 v3 格式 + 旧格式懒迁移）。
    pub fn get_config(
        conn: &Connection,
        machine_key: &[u8; 32],
    ) -> rusqlite::Result<SyncConfigDTO> {
        let (enabled, webdav_url, username, password_encrypted, remote_path, last_sync_at, last_sync_direction) = conn.query_row(
            "SELECT enabled, webdav_url, username, password_encrypted, remote_path, last_sync_at, last_sync_direction
             FROM sync_config WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )?;
        let password = password_encrypted
            .as_deref()
            .and_then(|e| decrypt_sync_password(e, machine_key, conn));
        Ok(SyncConfigDTO {
            enabled,
            webdav_url,
            username,
            password,
            remote_path,
            last_sync_at,
            last_sync_direction,
        })
    }

    /// 保存同步配置（密码以 v3 机器绑定格式加密写入）
    pub fn save_config(
        conn: &Connection,
        config: &SyncConfigDTO,
        machine_key: &[u8; 32],
    ) -> rusqlite::Result<()> {
        let password_encrypted = config
            .password
            .as_deref()
            .map(|p| encrypt_sync_password_v3(machine_key, p));
        conn.execute(
            "UPDATE sync_config SET
                enabled = ?1,
                webdav_url = ?2,
                username = ?3,
                password_encrypted = ?4,
                remote_path = ?5,
                last_sync_at = ?6,
                last_sync_direction = ?7
             WHERE id = 1",
            params![
                config.enabled,
                config.webdav_url,
                config.username,
                password_encrypted,
                config.remote_path,
                config.last_sync_at,
                config.last_sync_direction,
            ],
        )?;
        Ok(())
    }

    /// VACUUM INTO 生成 DB 一致快照到临时文件，返回文件路径。
    /// 快照是干净的单文件（不含 -wal/-shm），适合上传。
    fn create_snapshot(conn: &Connection, app_dir: &Path) -> Result<PathBuf, String> {
        let snapshot_path = app_dir.join("lumo_sync_snapshot.sqlite");
        // 先清理旧快照
        let _ = std::fs::remove_file(&snapshot_path);
        Self::remove_sidecar_files(&snapshot_path);
        let sql = format!(
            "VACUUM INTO '{}'",
            snapshot_path.to_string_lossy().replace('\'', "''")
        );
        conn.execute_batch(&sql)
            .map_err(|e| format!("Failed to create DB snapshot: {}", e))?;

        let mut snap_conn =
            Connection::open(&snapshot_path).map_err(|e| format!("无法打开生成的快照: {}", e))?;

        // MOB-005: 快照脱敏——清空所有可还原凭据，确保上传到云端的快照绝不携带密码。
        // 这一步失败必须中止上传：过去写成 `let _ =` 会让带凭据的快照照常发出去，
        // 等于把「凭据不上云」这条产品承诺建立在一次 UPDATE 必然成功的前提上。
        let sanitize = snap_conn
            .transaction()
            .and_then(|tx| {
                tx.execute("UPDATE sync_config SET password_encrypted = NULL", [])?;
                tx.execute(
                    "UPDATE sources SET credential_ref = NULL WHERE kind = 'webdav'",
                    [],
                )?;
                tx.commit()
            })
            .err()
            .map(|e| format!("快照凭据脱敏失败，已中止上传: {}", e));
        if let Some(err) = sanitize {
            let _ = std::fs::remove_file(&snapshot_path);
            return Err(err);
        }

        // 上传前自检：本地库已经损坏时，在这里失败远好过在用户最需要恢复的时候失败。
        let integrity: String = snap_conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| format!("快照完整性检查失败: {}", e))?;
        // 同上：脱敏与检查的连接必须先释放，再删 -wal/-shm。
        drop(snap_conn);
        Self::remove_sidecar_files(&snapshot_path);
        if !integrity.eq_ignore_ascii_case("ok") {
            let _ = std::fs::remove_file(&snapshot_path);
            return Err(format!("快照完整性检查未通过，已中止上传: {}", integrity));
        }

        Ok(snapshot_path)
    }

    /// 构建 WebdavClient（从 sync_config 的 URL + 凭据）
    fn build_client(config: &SyncConfigDTO) -> Result<WebdavClient, String> {
        let url = config.webdav_url.as_deref().ok_or("WebDAV URL 未配置")?;
        let username = config.username.clone();
        let password = config.password.clone();
        Ok(WebdavClient::new(url.to_string(), username, password))
    }

    /// 构建远程文件完整 URL（webserver root + remote_path + filename）
    fn remote_file_url(config: &SyncConfigDTO, filename: &str) -> Result<String, String> {
        let base_url = config.webdav_url.as_deref().ok_or("WebDAV URL 未配置")?;
        let remote_dir = config.remote_path.as_deref().unwrap_or("/");
        let remote_dir = remote_dir.trim_end_matches('/');
        let base =
            reqwest::Url::parse(base_url).map_err(|e| format!("无效的 WebDAV URL: {}", e))?;
        // 构建完整路径
        let joined = base
            .join(&format!("{}/{}", remote_dir, filename))
            .map_err(|e| format!("路径组合失败: {}", e))?;
        Ok(joined.to_string())
    }

    /// 上传同步快照：VACUUM INTO → PUT 到 remote_path 的临时名 → 校验和 sidecar → MOVE 覆盖正式名。
    ///
    /// 为什么绕这一圈：PUT 直接覆盖 `lumo.sqlite` 时，若网络在中途断开，
    /// 远端会留下半截快照，而下一次「从云端恢复」会把这份损坏数据整库灌进本地。
    /// 先写临时名、再 MOVE 替换，让正式名要么完整、要么是上一次的完整版本。
    /// 不支持 MOVE 的服务器退回直接 PUT（见 `WEBDAV_COMPATIBILITY.md`），
    /// 此时兜底防线是恢复侧的 SQLite 文件头 + integrity_check 校验。
    pub fn sync_upload(
        conn: &Connection,
        app_dir: &Path,
        config: &SyncConfigDTO,
    ) -> Result<SyncResult, String> {
        let client = Self::build_client(config)?;
        let snapshot = Self::create_snapshot(conn, app_dir)?;
        let file_size = std::fs::metadata(&snapshot)
            .map_err(|e| e.to_string())?
            .len();
        let checksum = sha256_of_file(&snapshot)?;
        let data = std::fs::read(&snapshot).map_err(|e| format!("读取快照失败: {}", e))?;

        // 确保远程目录存在
        let remote_dir = config.remote_path.as_deref().unwrap_or("/");
        if remote_dir != "/" && !remote_dir.is_empty() {
            let dir_url = Self::remote_file_url(config, "")?;
            let _ = client.mkcol(&dir_url); // 忽略 405 已存在
        }

        let upload_url = Self::remote_file_url(config, REMOTE_SNAPSHOT_NAME)?;
        let checksum_url = Self::remote_file_url(config, REMOTE_CHECKSUM_NAME)?;
        let staging_name = format!("{}.tmp-{}", REMOTE_SNAPSHOT_NAME, staging_suffix());
        let staging_url = Self::remote_file_url(config, &staging_name)?;

        let upload_result = (|| -> Result<(), String> {
            client.put_file(&staging_url, data.clone())?;
            // 校验和先于正式名就位：任何时刻远端的 `lumo.sqlite` 都不会配上更新的校验和，
            // 最坏情况是校验和比库新（恢复侧按「不匹配」拒绝，重新备份一次即可自愈），
            // 而不是校验和比库旧（会放过损坏数据）。
            client.put_file(&checksum_url, checksum.as_bytes().to_vec())?;
            if let Err(e) = client.move_file(&staging_url, &upload_url) {
                tracing::warn!("WebDAV MOVE 不可用，退回直接 PUT 覆盖：{}", e);
                let _ = client.delete(&staging_url);
                client.put_file(&upload_url, data)?;
            }
            Ok(())
        })();

        // 本地快照无论成败都清掉：它是完整库的副本，留在数据目录里只会被误读
        let _ = std::fs::remove_file(&snapshot);
        Self::remove_sidecar_files(&snapshot);
        upload_result?;

        use chrono::Utc;
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 更新 last_sync_at
        conn.execute(
            "UPDATE sync_config SET last_sync_at = ?1, last_sync_direction = 'upload' WHERE id = 1",
            params![timestamp],
        )
        .map_err(|e| format!("更新同步时间失败: {}", e))?;

        Ok(SyncResult {
            bytes_uploaded: file_size,
            timestamp,
        })
    }

    /// 下载云端快照到本地唯一临时文件，返回文件路径。
    /// 调用方负责校验、替换 DB 与清理。临时名带随机后缀：固定名会让上一次半途失败的残留
    /// 文件被这一次当成「刚下载好的内容」直接恢复。
    pub fn sync_download_to_temp(
        app_dir: &Path,
        config: &SyncConfigDTO,
    ) -> Result<PathBuf, String> {
        Self::cleanup_stale_downloads(app_dir);
        let client = Self::build_client(config)?;
        let download_url = Self::remote_file_url(config, REMOTE_SNAPSHOT_NAME)?;
        let temp_path = app_dir.join(format!(
            "{}.{}.download",
            REMOTE_SNAPSHOT_NAME,
            staging_suffix()
        ));
        client.download_to_file(&download_url, &temp_path)?;

        // 校验和 sidecar 存在则比对；不存在（老版本备份、服务器拒绝）跳过，
        // 由 validate_snapshot 的结构校验兜底。
        let checksum_url = Self::remote_file_url(config, REMOTE_CHECKSUM_NAME)?;
        match client.fetch_text(&checksum_url, 256) {
            Ok(Some(text)) => match parse_checksum(&text) {
                Some(expected) => {
                    let actual = sha256_of_file(&temp_path)?;
                    if actual != expected {
                        let _ = std::fs::remove_file(&temp_path);
                        return Err(
                            "云端快照校验失败（下载内容不完整或已被改写），已拒绝恢复".to_string()
                        );
                    }
                }
                None => {
                    tracing::warn!("云端校验和文件格式异常，跳过校验和比对");
                }
            },
            Ok(None) => {}
            Err(e) => tracing::warn!("读取云端校验和失败（{}），跳过校验和比对", e),
        }

        Ok(temp_path)
    }

    /// 检查云端是否有同步数据（用于首次配置检测）
    pub fn check_remote(
        config: &SyncConfigDTO,
    ) -> Result<crate::models::RemoteCheckResult, String> {
        let client = Self::build_client(config)?;
        let remote_dir = config.remote_path.as_deref().unwrap_or("/");
        let path = if remote_dir == "/" || remote_dir.is_empty() {
            "/"
        } else {
            remote_dir.trim_end_matches('/')
        };
        let files = client.propfind(path)?;

        // 查找 lumo.sqlite
        for f in &files {
            if f.path.ends_with(REMOTE_SNAPSHOT_NAME) {
                return Ok(crate::models::RemoteCheckResult {
                    has_data: true,
                    remote_size: Some(f.size),
                    last_modified: Some(f.last_modified.clone()),
                });
            }
        }
        Ok(crate::models::RemoteCheckResult {
            has_data: false,
            remote_size: None,
            last_modified: None,
        })
    }

    /// 浏览 WebDAV 目录：返回某路径下的子目录列表（PROPFIND 过滤后仅目录）
    pub fn browse(config: &SyncConfigDTO, path: &str) -> Result<Vec<WebdavFile>, String> {
        let client = Self::build_client(config)?;
        let files = client.propfind(path)?;
        // 排除自身条目，只返回目录
        Ok(files.into_iter().filter(|f| f.is_dir).collect())
    }

    /// 在 WebDAV 上创建目录
    pub fn create_remote_folder(config: &SyncConfigDTO, path: &str) -> Result<(), String> {
        let client = Self::build_client(config)?;
        client.mkcol(path)
    }
}

/// 流式计算文件 SHA-256（十六进制小写）。快照可能有上百 MB，不能整体读进内存。
fn sha256_of_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = File::open(path).map_err(|e| format!("无法读取快照文件: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("读取快照文件失败: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 解析远端校验和文件：只接受 64 位十六进制（允许尾随空白）。
/// 服务器返回 HTML 错误页、目录页时被识别为「无校验和」而不是拿乱码去比对。
fn parse_checksum(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?;
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(token.to_lowercase())
}

/// 临时文件后缀：纳秒时间戳 + 进程号，同一秒内多次操作也不会撞名。
fn staging_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}", std::process::id(), nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_checksum_accepts_plain_and_sum_format() {
        let digest = "a".repeat(64);
        assert_eq!(
            parse_checksum(&format!("{}\n", digest)),
            Some(digest.clone())
        );
        // `sha256sum` 输出格式（"hash  filename"）同样接受
        assert_eq!(
            parse_checksum(&format!("{}  lumo.sqlite", digest)),
            Some(digest)
        );
    }

    #[test]
    fn parse_checksum_rejects_non_checksum_bodies() {
        assert_eq!(parse_checksum("<html><body>404</body></html>"), None);
        assert_eq!(parse_checksum(""), None);
        assert_eq!(parse_checksum(&"z".repeat(64)), None);
        assert_eq!(parse_checksum(&"a".repeat(63)), None);
    }

    /// DATA-002：恢复前的快照甄别。
    /// 这里的每一种输入都是"恢复"这条链路上会真的覆盖本地库的输入，
    /// 断言它们被拒绝，等于断言用户不会因为远端一份坏文件而丢曲库。
    mod snapshot_validation {
        use crate::db::test_util::TempDir;
        use rusqlite::Connection;

        /// 生成一份「本机刚备份出去」的快照。
        fn snapshot_in(dir: &TempDir) -> std::path::PathBuf {
            let path = dir.db_path();
            crate::db::init_db(path.clone()).expect("构造测试库失败");
            path
        }

        #[test]
        fn accepts_a_current_lumo_database() {
            let dir = TempDir::new("snap_current");
            let path = snapshot_in(&dir);
            super::SyncService::validate_snapshot(&path).expect("合法快照应通过校验");
        }

        #[test]
        fn migrates_a_historical_schema_before_accepting() {
            let dir = TempDir::new("snap_history");
            let path = snapshot_in(&dir);
            // 退回到"旧版本备份"：schema 停在 V8，版本号也只剩 1..=8
            let conn = Connection::open(&path).unwrap();
            conn.execute("DELETE FROM schema_migrations WHERE version > 8", [])
                .unwrap();
            drop(conn);

            super::SyncService::validate_snapshot(&path).expect("历史快照应先迁移再接受");

            let conn = Connection::open(&path).unwrap();
            let version: i64 = conn
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(
                version,
                crate::db::TARGET_SCHEMA_VERSION,
                "恢复前必须把历史 schema 迁到本机版本"
            );
        }

        #[test]
        fn rejects_html_error_page_and_empty_file() {
            let dir = TempDir::new("snap_garbage");

            let html = dir.path().join("lumo_gw.html");
            std::fs::write(
                &html,
                b"<!DOCTYPE html><html><head><title>Gateway</title></head></html>",
            )
            .unwrap();
            let err = super::SyncService::validate_snapshot(&html)
                .expect_err("网关错误页绝不能被当成数据库恢复");
            assert!(err.contains("不是有效的 SQLite"), "文案失真: {}", err);

            let empty = dir.path().join("lumo_empty.sqlite");
            std::fs::write(&empty, []).unwrap();
            assert!(super::SyncService::validate_snapshot(&empty).is_err());
        }

        #[test]
        fn rejects_other_software_database() {
            let dir = TempDir::new("snap_foreign");
            let path = dir.path().join("lumo_foreign.sqlite");
            // 有效 SQLite 文件，但不是 Lumo 的库：没有 schema_migrations
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);")
                .unwrap();
            drop(conn);

            let err = super::SyncService::validate_snapshot(&path)
                .expect_err("外部数据库绝不能覆盖本地曲库");
            assert!(err.contains("不是 Lumo 数据库"), "文案失真: {}", err);
            // 关键：拒绝不得以"顺手建表"的方式把它加工成 Lumo 库
            let conn = Connection::open(&path).unwrap();
            let created: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='tracks')",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(!created, "校验失败前不得对快照做任何写入");
        }

        #[test]
        fn rejects_snapshot_from_a_newer_app_version() {
            let dir = TempDir::new("snap_future");
            let path = snapshot_in(&dir);
            let conn = Connection::open(&path).unwrap();
            conn.execute("INSERT INTO schema_migrations (version) VALUES (99)", [])
                .unwrap();
            drop(conn);

            let err = super::SyncService::validate_snapshot(&path)
                .expect_err("来自更新版本的快照不能强行降级恢复");
            assert!(err.contains("更新版本"), "文案失真: {}", err);
        }

        #[test]
        fn rejects_truncated_snapshot_and_leaves_no_wal_behind() {
            let dir = TempDir::new("snap_truncated");
            let path = snapshot_in(&dir);
            // 半截下载：截掉后一半页（保留合法文件头，骗过 header 魔数检查）
            let full = std::fs::read(&path).unwrap();
            std::fs::write(&path, &full[..full.len() / 2]).unwrap();

            let err = super::SyncService::validate_snapshot(&path)
                .expect_err("截断的快照必须被拒绝，而不是恢复出一个残缺曲库");
            assert!(!err.is_empty());

            // 校验过程以 WAL 打开过快照；残留的 -wal 会让下一次恢复读到过期页
            assert!(!dir.path().join("lumo.sqlite-wal").exists());
        }

        #[test]
        fn staging_names_are_unique_and_checksum_format_is_stable() {
            assert_ne!(super::staging_suffix(), super::staging_suffix());
            let dir = TempDir::new("snap_hash");
            let path = snapshot_in(&dir);
            let digest = super::sha256_of_file(&path).unwrap();
            assert_eq!(digest.len(), 64);
            assert_eq!(super::parse_checksum(&digest), Some(digest.clone()));
            // 同一文件重算一致；改动一位则不同
            std::fs::write(dir.path().join("copy"), "x").unwrap();
            assert_ne!(
                digest,
                super::sha256_of_file(&dir.path().join("copy")).unwrap()
            );
        }
    }
}

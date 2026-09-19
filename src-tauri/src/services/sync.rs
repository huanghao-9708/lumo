use crate::models::{SyncConfigDTO, SyncResult};
use crate::services::webdav::{WebdavClient, WebdavFile};
use chrono::{DateTime, Utc};
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

/// 远端暂存对象的最短存活期（24 小时），见 [`is_stale_staging`]。
///
/// 暂存名后缀是设备本地的「进程号+纳秒」，一台设备无从判断远端那份 `tmp-*` 是不是
/// 另一台设备正在上传的半成品，所以只能按 mtime 划界。24 小时远大于一次备份
/// （即便 2 GiB 走慢速链路）的合理耗时，超过它就可以认定是残留。
const STALE_STAGING_MIN_AGE_SECS: i64 = 24 * 3600;

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
        // 联网行为: §2F —— 地址来自 sync_config（用户自填）
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

        // 确保远程目录存在
        let remote_dir = config.remote_path.as_deref().unwrap_or("/");
        if remote_dir != "/" && !remote_dir.is_empty() {
            let dir_url = Self::remote_file_url(config, "")?;
            let _ = client.mkcol(&dir_url); // 忽略 405 已存在
        }
        // 上一版本 / 其它设备半途失败留下的暂存名，开工前先收掉（CR-003）
        Self::cleanup_stale_staging(&client, remote_dir);

        let upload_url = Self::remote_file_url(config, REMOTE_SNAPSHOT_NAME)?;
        let checksum_url = Self::remote_file_url(config, REMOTE_CHECKSUM_NAME)?;
        let staging_name = remote_staging_name(&staging_suffix());
        let staging_url = Self::remote_file_url(config, &staging_name)?;

        // 暂存名的清理不写在失败分支里，而是交给守卫：校验和 PUT 失败、MOVE 之后的
        // 降级 PUT 失败、甚至 panic，都不该在用户配额里永久留下一份完整数据库副本（CR-003）。
        let mut staging =
            // 联网行为: §2F
            RemoteStagingGuard::new(|url| client.delete(url), staging_url.clone(), staging_name);

        let upload_result = (|| -> Result<(), String> {
            // 每次 PUT 各自重新打开快照文件流式上传：内存里不同时存在两份整库字节（CR-004）
            // 体积一并传给上传接口换算总时间预算：服务器接了连接却不再收字节时，
            // 请求必须在一个明确的时间点返回而不是挂到永远（CR-007）
            client.put_file(&staging_url, open_snapshot(&snapshot)?, Some(file_size))?;
            // 校验和先于正式名就位：任何时刻远端的 `lumo.sqlite` 都不会配上更新的校验和，
            // 最坏情况是校验和比库新（恢复侧按「不匹配」拒绝，重新备份一次即可自愈），
            // 而不是校验和比库旧（会放过损坏数据）。
            client.put_file(
                &checksum_url,
                checksum.as_bytes().to_vec(),
                Some(checksum.len() as u64),
            )?;
            match client.move_file(&staging_url, &upload_url) {
                Ok(()) => staging.disarm(),
                Err(e) => {
                    tracing::warn!("WebDAV MOVE 不可用，退回直接 PUT 覆盖：{}", e);
                    // 显式删暂存名（守卫随之解除），降级路径不留第二份
                    if let Some(detail) = staging.cleanup() {
                        tracing::warn!("远端暂存文件 {} 清理失败：{}", staging.name(), detail);
                    }
                    client.put_file(&upload_url, open_snapshot(&snapshot)?, Some(file_size))?;
                }
            }
            Ok(())
        })();
        // 成功与降级路径都会走到这里：守卫已解除时返回 None，不会多发一次 DELETE
        let residue = staging.cleanup();

        // 本地快照无论成败都清掉：它是完整库的副本，留在数据目录里只会被误读
        let _ = std::fs::remove_file(&snapshot);
        Self::remove_sidecar_files(&snapshot);
        upload_result.map_err(|e| match residue {
            Some(detail) => staging_residue_message(&e, staging.name(), &detail),
            None => e,
        })?;

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

    /// 尽力清理远端目录里**本应用**的陈旧暂存对象（`lumo.sqlite.tmp-*`，见 CR-003）。
    ///
    /// 为什么只清「旧到一定年纪」的：暂存名里带的是设备本地的 pid+纳秒，跨设备无从判断
    /// 是不是别人正在传的那一份；用 mtime 划界才能保证不会误删另一台设备进行中的上传。
    /// PROPFIND 失败、删除失败都只记日志——清理不该把一次正常备份变成失败。
    fn cleanup_stale_staging(client: &WebdavClient, remote_dir: &str) {
        let files = match client.propfind(remote_dir) {
            Ok(files) => files,
            Err(e) => {
                tracing::debug!("清理远端暂存文件前 PROPFIND 失败，跳过清理：{}", e);
                return;
            }
        };
        let now = Utc::now();
        for file in files {
            if file.is_dir {
                continue;
            }
            let name = file_basename(&file.path);
            if !is_stale_staging(&name, &file.last_modified, now, STALE_STAGING_MIN_AGE_SECS) {
                continue;
            }
            let url = match client.build_url(&file.path) {
                Ok(url) => url,
                Err(e) => {
                    tracing::warn!("远端暂存文件地址无法解析，跳过 {}：{}", name, e);
                    continue;
                }
            };
            // 联网行为: §2F
            match client.delete(&url) {
                Ok(()) => tracing::info!("已清理陈旧的远端暂存文件 {}", name),
                Err(e) => tracing::warn!("远端暂存文件 {} 清理失败：{}", name, e),
            }
        }
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
        // 体积未知（开工前拿不到远端大小）：走体积未知的兜底总预算，停滞的服务器同样会在
        // 明确时间内返回（CR-007）。半途失败必须立刻删掉临时文件——下一轮开头的
        // cleanup_stale_downloads 只是兜底，不能拿它当清理手段，否则用户配额里会一直
        // 躺着一份半截数据库快照。
        if let Err(e) = client.download_to_file(&download_url, &temp_path, None) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(e);
        }

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

/// 远端暂存对象名：正式快照名 + `tmp-` + 设备本地唯一后缀。
fn remote_staging_name(suffix: &str) -> String {
    format!("{}.tmp-{}", REMOTE_SNAPSHOT_NAME, suffix)
}

/// 本应用暂存对象的共同前缀，用于在远端目录里认出「这是我们遗留的半成品」。
fn staging_prefix() -> String {
    format!("{}.tmp-", REMOTE_SNAPSHOT_NAME)
}

/// 以只读方式打开快照，交给 reqwest 流式 PUT（CR-004）。
///
/// 每次调用都新开一个句柄：暂存 PUT 与降级 PUT 各自独立读盘，
/// 内存里因此不会同时存在两份完整数据库的字节。
fn open_snapshot(path: &Path) -> Result<File, String> {
    File::open(path).map_err(|e| format!("读取快照失败: {}", e))
}

/// 从 PROPFIND 返回的路径（可能是相对子路径，也可能是完整 URL）中取出对象名。
fn file_basename(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_string()
}

/// 判断远端对象是否是本应用可以安全清理的陈旧暂存文件（CR-003）。
///
/// 三个条件缺一不可：名字符合本应用的暂存格式、服务器给出了时间、且已超过最短存活期。
/// 时间缺失、格式无法解析、或落在未来（服务器时钟不准）一律返回 false——
/// 误删另一台设备进行中的上传，代价远大于多在配额里留一个残留。
fn is_stale_staging(
    name: &str,
    last_modified: &str,
    now: DateTime<Utc>,
    min_age_secs: i64,
) -> bool {
    let Some(suffix) = name.strip_prefix(&staging_prefix()) else {
        return false;
    };
    if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return false;
    }
    let Some(mtime) = parse_remote_timestamp(last_modified) else {
        return false;
    };
    // 未来时间（时钟漂移）走的是有符号减法，结果为负，自然落在「不删」一侧
    now.signed_duration_since(mtime).num_seconds() > min_age_secs
}

/// 解析 WebDAV `getlastmodified`：规范值是 HTTP-date（RFC 1123），
/// 但不少服务器返回 RFC 3339 或裸时间戳，逐个尝试，解析不出就当作没有时间信息。
fn parse_remote_timestamp(text: &str) -> Option<DateTime<Utc>> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(parsed) = DateTime::parse_from_rfc2822(text) {
        return Some(parsed.with_timezone(&Utc));
    }
    if let Ok(parsed) = DateTime::parse_from_rfc3339(text) {
        return Some(parsed.with_timezone(&Utc));
    }
    chrono::NaiveDateTime::parse_from_str(text, "%a, %d %b %Y %H:%M:%S %Z")
        .ok()
        .map(|naive| naive.and_utc())
}

/// 上传失败、且远端暂存对象没能一起删掉时的用户可见文案（CR-003 验收：错误里要有对象名）。
///
/// 暂存名是随机生成的，用户无法从别处得知它叫什么；没有这个名字，服务器上那条残留
/// 就成了一堆 `tmp-*` 里认不出来的孤儿，只能整目录删。
fn staging_residue_message(cause: &str, name: &str, detail: &str) -> String {
    format!(
        "{}（远端暂存文件 {} 也未能删除，请在服务器上手动清理：{}）",
        cause, name, detail
    )
}

/// 远端暂存对象的作用域清理守卫（CR-003）。
///
/// 上传链路上任何一步失败——校验和 PUT 失败、MOVE 后的降级 PUT 失败、甚至中途 panic——
/// 都必须消掉已经写上去的那份完整数据库副本：它白占用户的 WebDAV 配额，里面还是可恢复的私人数据。
/// 只有 MOVE 成功（暂存名已改名成正式名）才解除守卫，那时再 DELETE 会把刚备份好的库删掉。
struct RemoteStagingGuard<D>
where
    D: FnMut(&str) -> Result<(), String>,
{
    delete: D,
    url: String,
    name: String,
    armed: bool,
}

impl<D> RemoteStagingGuard<D>
where
    D: FnMut(&str) -> Result<(), String>,
{
    fn new(delete: D, url: String, name: String) -> Self {
        Self {
            delete,
            url,
            name,
            armed: true,
        }
    }

    /// 暂存名已成功替换为正式名，后续不得再删除。
    fn disarm(&mut self) {
        self.armed = false;
    }

    /// 尽力删除暂存对象；已解除或已清理过则什么都不做（幂等）。
    /// 返回删除失败的原因，调用方把对象名拼进用户可见文案。
    fn cleanup(&mut self) -> Option<String> {
        if !self.armed {
            return None;
        }
        self.armed = false;
        let url = self.url.clone();
        (self.delete)(&url).err()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl<D> Drop for RemoteStagingGuard<D>
where
    D: FnMut(&str) -> Result<(), String>,
{
    fn drop(&mut self) {
        // 走到这里说明调用方没能走到显式清理那一步（提前 return 或 panic）：
        // 尽力删除，失败只记日志——正在掉栈的路径上再抛错误只会盖掉真正的原因。
        if let Some(detail) = self.cleanup() {
            tracing::warn!(
                "远端暂存文件 {} 未能自动清理，请到服务器上手动删除：{}",
                self.name,
                detail
            );
        }
    }
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

    /// CR-003：远端暂存对象必须在上传链路的**任何**失败分支上被收掉。
    /// 用假 delete 记录调用，不依赖真实 WebDAV 服务；断言的是「发了几次 DELETE、
    /// 在什么状态下不发」，正好对应报告验收标准里的三条。
    mod remote_staging_cleanup {
        use super::super::{
            file_basename, is_stale_staging, parse_remote_timestamp, remote_staging_name,
            staging_prefix, staging_residue_message, staging_suffix, RemoteStagingGuard,
        };
        use chrono::{DateTime, TimeZone, Utc};
        use std::cell::RefCell;
        use std::rc::Rc;

        const STAGING_URL: &str = "https://dav.example/backup/lumo.sqlite.tmp-7-8";
        const STAGING_NAME: &str = "lumo.sqlite.tmp-7-8";

        /// 记录每次 DELETE 的假客户端；`fail` 模拟服务器拒绝删除（403 / 网络断开）。
        fn fake_delete(
            calls: Rc<RefCell<Vec<String>>>,
            fail: bool,
        ) -> impl FnMut(&str) -> Result<(), String> {
            move |url: &str| {
                calls.borrow_mut().push(url.to_string());
                if fail {
                    Err("服务器拒绝删除 (403)".to_string())
                } else {
                    Ok(())
                }
            }
        }

        fn guard(
            calls: Rc<RefCell<Vec<String>>>,
            fail: bool,
        ) -> RemoteStagingGuard<impl FnMut(&str) -> Result<(), String>> {
            RemoteStagingGuard::new(
                fake_delete(calls, fail),
                STAGING_URL.to_string(),
                STAGING_NAME.to_string(),
            )
        }

        /// 校验和 PUT 失败（上传半途中止）：已经传上去的整库副本必须被删掉，且只删一次。
        #[test]
        fn cleanup_deletes_the_staged_snapshot_exactly_once() {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let mut staging = guard(calls.clone(), false);
            assert_eq!(staging.cleanup(), None, "删除成功时不该有残留详情");
            // 收尾清理 + Drop 都会再问一次：幂等，不得重复发 DELETE
            assert_eq!(staging.cleanup(), None);
            drop(staging);
            assert_eq!(calls.borrow().as_slice(), [STAGING_URL.to_string()]);
        }

        /// MOVE 成功后暂存名已经变成正式快照：此时再 DELETE 等于把刚备份好的库删了。
        #[test]
        fn disarming_after_move_forbids_every_later_delete() {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let mut staging = guard(calls.clone(), false);
            staging.disarm();
            assert_eq!(staging.cleanup(), None);
            drop(staging);
            assert!(calls.borrow().is_empty(), "已解除的守卫不得发出 DELETE");
        }

        /// MOVE 不可用时的降级路径：显式清理一次，收尾与 Drop 都不该再来一次。
        #[test]
        fn fallback_path_cleanup_happens_once() {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let mut staging = guard(calls.clone(), false);
            if let Some(detail) = staging.cleanup() {
                panic!("删除不该失败：{}", detail);
            }
            let residue = staging.cleanup();
            assert!(residue.is_none());
            drop(staging);
            assert_eq!(calls.borrow().len(), 1);
        }

        /// 提前 return / panic 兜底：没走到显式清理时，Drop 仍要把远端半成品收掉。
        #[test]
        fn drop_alone_still_removes_the_staged_snapshot() {
            let calls = Rc::new(RefCell::new(Vec::new()));
            {
                let _staging = guard(calls.clone(), false);
                // 模拟上传中途 `?` 直接返回，守卫未参与收尾
            }
            assert_eq!(calls.borrow().as_slice(), [STAGING_URL.to_string()]);
        }

        /// 删除也失败时，守卫要把原因交回调用方，由用户可见文案带上对象名。
        #[test]
        fn delete_failure_surfaces_the_object_name_to_the_user() {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let mut staging = guard(calls.clone(), true);
            let detail = staging
                .cleanup()
                .expect("删除失败必须返回原因，否则用户无从手工清理");
            assert!(detail.contains("403"), "原因要能看出为何失败：{}", detail);
            let message = staging_residue_message("上传失败：连接断开", staging.name(), &detail);
            assert!(message.contains(STAGING_NAME), "文案必须点名远端对象");
            assert!(message.contains("403"));
            assert!(calls.borrow().len() == 1);
        }

        /// 上传成功但清理失败时不得把成功说成失败：residue 只在上传本身失败时才拼进文案。
        #[test]
        fn residue_message_is_only_for_failed_uploads() {
            let ok: Result<(), String> = Ok(());
            let mapped = ok.map_err(|e| staging_residue_message(&e, STAGING_NAME, "ignored"));
            assert!(mapped.is_ok());
        }

        fn http_date(dt: DateTime<Utc>) -> String {
            dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
        }

        fn now() -> DateTime<Utc> {
            Utc.with_ymd_and_hms(2026, 9, 19, 12, 0, 0).unwrap()
        }

        /// 只有「本应用暂存名 + 有可信时间 + 超过最短存活期」三者齐备才允许自动删除。
        #[test]
        fn only_our_own_stale_staging_names_are_swept() {
            let stale = http_date(now() - chrono::Duration::seconds(25 * 3600));
            let fresh = http_date(now() - chrono::Duration::seconds(60));
            assert!(is_stale_staging(STAGING_NAME, &stale, now(), 24 * 3600));
            // 另一台设备正在上传的那一份绝不能碰
            assert!(!is_stale_staging(STAGING_NAME, &fresh, now(), 24 * 3600));
            // 正式快照名与校验和 sidecar 都不属于暂存对象
            assert!(!is_stale_staging("lumo.sqlite", &stale, now(), 24 * 3600));
            assert!(!is_stale_staging(
                "lumo.sqlite.sha256",
                &stale,
                now(),
                24 * 3600
            ));
            // 别的客户端恰好也用了 tmp- 命名
            assert!(!is_stale_staging(
                "other-app.sqlite.tmp-1",
                &stale,
                now(),
                24 * 3600
            ));
            // 后缀形状不对（不是 pid-nanos）也一律不认
            assert!(!is_stale_staging(
                "lumo.sqlite.tmp-",
                &stale,
                now(),
                24 * 3600
            ));
            assert!(!is_stale_staging(
                "lumo.sqlite.tmp-notes",
                &stale,
                now(),
                24 * 3600
            ));
        }

        /// 时间信息缺失、乱码或落在未来（服务器时钟不准）时一律不删：宁可留残留。
        #[test]
        fn untrusted_timestamps_never_trigger_deletion() {
            let stale = 25 * 3600;
            assert!(!is_stale_staging(STAGING_NAME, "", now(), stale));
            assert!(!is_stale_staging(STAGING_NAME, "not-a-date", now(), stale));
            assert!(!is_stale_staging(STAGING_NAME, "0", now(), stale));
            let future = http_date(now() + chrono::Duration::seconds(stale));
            assert!(!is_stale_staging(STAGING_NAME, &future, now(), stale));
        }

        #[test]
        fn remote_timestamps_accept_http_and_iso_formats() {
            assert_eq!(
                parse_remote_timestamp("Sat, 19 Sep 2026 11:00:00 GMT"),
                Some(now() - chrono::Duration::seconds(3600))
            );
            assert_eq!(
                parse_remote_timestamp("2026-09-19T12:00:00+00:00"),
                Some(now())
            );
            assert_eq!(parse_remote_timestamp("  "), None);
        }

        /// 上传侧与清理侧必须用同一个名字格式：格式漂了就等于永远清不掉残留。
        #[test]
        fn staging_name_format_stays_recognizable() {
            let name = remote_staging_name(&staging_suffix());
            assert!(name.starts_with(&staging_prefix()));
            let stale = http_date(now() - chrono::Duration::seconds(25 * 3600));
            assert!(is_stale_staging(&name, &stale, now(), 24 * 3600));
        }

        #[test]
        fn basename_handles_relative_paths_urls_and_trailing_slashes() {
            assert_eq!(
                file_basename("/backup/lumo.sqlite.tmp-1-2"),
                "lumo.sqlite.tmp-1-2"
            );
            assert_eq!(
                file_basename("https://dav.example/backup/lumo.sqlite.tmp-1-2"),
                "lumo.sqlite.tmp-1-2"
            );
            assert_eq!(file_basename("lumo.sqlite.tmp-1-2"), "lumo.sqlite.tmp-1-2");
            assert_eq!(file_basename("/backup/snapshots/"), "snapshots");
        }
    }

    /// CR-004：快照上传的内存峰值不得随库大小线性翻倍。
    /// 落地方式是把 `File` 直接交给 reqwest（`Body: From<File>`，按 metadata 推 content-length），
    /// 因此这里能自动化验证的关键事实是：请求体是文件句柄、长度来自文件系统，
    /// 而不是进程里的一份 `Vec<u8>`。真实 1 GiB 峰值内存仍需人工基线。
    #[test]
    fn snapshot_body_is_file_backed_and_carries_its_length() {
        use crate::db::test_util::TempDir;
        let dir = TempDir::new("upload_stream");
        let path = dir.path().join("lumo.sqlite.tmp");
        let bytes = vec![0x5a_u8; 4 * 1024 * 1024];
        let len = bytes.len() as u64;
        std::fs::write(&path, &bytes).unwrap();

        let file = super::open_snapshot(&path).expect("快照应能只读打开");
        assert_eq!(
            file.metadata().unwrap().len(),
            len,
            "交给请求体的必须是磁盘上的完整快照"
        );
        let body: reqwest::blocking::Body = file.into();
        drop(body);
        // 请求体只是句柄：读盘动作发生在发送时，转换本身既不占用也不消耗本地快照
        assert_eq!(std::fs::metadata(&path).unwrap().len(), len);
        let again = super::open_snapshot(&path).unwrap();
        assert_eq!(again.metadata().unwrap().len(), len);
    }
}

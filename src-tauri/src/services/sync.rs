use crate::models::{SyncConfigDTO, SyncResult};
use crate::services::webdav::{WebdavClient, WebdavFile};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

/// 旧版同步密钥（硬编码，全设备相同）——仅用于读取历史存量数据（P1-08），
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
    let bytes: Vec<u8> = password.bytes()
        .enumerate()
        .map(|(i, b)| b ^ machine_key[i % 32])
        .collect();
    format!("v3:seal:{}", base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// 解密同步密码：按前缀分发 v3（机器绑定）/ 旧格式（硬编码 key）。
/// 旧格式解密成功时同步触发懒迁移（UPDATE 为 v3），返回 (密码, 是否发生迁移)。
fn decrypt_sync_password(encoded: &str, machine_key: &[u8; 32], conn: &Connection) -> Option<String> {
    use base64::Engine;
    if let Some(payload) = encoded.strip_prefix("v3:seal:") {
        let bytes = base64::engine::general_purpose::STANDARD.decode(payload).ok()?;
        let decrypted: Vec<u8> = bytes.iter()
            .enumerate()
            .map(|(i, b)| b ^ machine_key[i % 32])
            .collect();
        return String::from_utf8(decrypted).ok();
    }

    // 旧格式：硬编码 key
    let key = derive_sync_key();
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).ok()?;
    let decrypted: Vec<u8> = bytes.iter()
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
    /// Validate and migrate a downloaded snapshot before it can touch the live database.
    pub fn validate_snapshot(path: &Path, app_dir: &Path) -> Result<(), String> {
        let _ = crate::db::init_db(path.to_path_buf())
            .map_err(|e| format!("无法初始化同步数据库快照: {}", e))?;
        let conn = Connection::open(path)
            .map_err(|e| format!("无法打开同步数据库快照: {}", e))?;
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| format!("同步数据库完整性检查失败: {}", e))?;
        if !integrity.eq_ignore_ascii_case("ok") {
            return Err(format!("同步数据库完整性检查未通过: {}", integrity));
        }

        let required_tables = ["schema_migrations", "sources", "tracks", "media_files", "sync_config"];
        for table in required_tables {
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get(0),
            ).map_err(|e| format!("检查同步数据库结构失败: {}", e))?;
            if !exists {
                return Err(format!("同步数据库缺少必要表: {}", table));
            }
        }

        let version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        ).map_err(|e| format!("读取同步数据库版本失败: {}", e))?;
        if version < 8 {
            return Err(format!("同步数据库版本过旧（{}），需要至少 V8", version));
        }
        let _ = app_dir;
        Ok(())
    }

    /// 读取同步配置（密码解密后返回）。需要机器密钥（P1-08 v3 格式 + 旧格式懒迁移）。
    pub fn get_config(conn: &Connection, machine_key: &[u8; 32]) -> rusqlite::Result<SyncConfigDTO> {
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
    pub fn save_config(conn: &Connection, config: &SyncConfigDTO, machine_key: &[u8; 32]) -> rusqlite::Result<()> {
        let password_encrypted = config.password.as_deref().map(|p| encrypt_sync_password_v3(machine_key, p));
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
        let sql = format!("VACUUM INTO '{}'", snapshot_path.to_string_lossy().replace('\'', "''"));
        conn.execute_batch(&sql).map_err(|e| format!("Failed to create DB snapshot: {}", e))?;

        // MOB-005: 快照脱敏处理：清空密码字段，确保上传至云端的快照绝不携带可还原密码
        if let Ok(snap_conn) = Connection::open(&snapshot_path) {
            let _ = snap_conn.execute_batch("
                UPDATE sync_config SET password_encrypted = NULL;
                UPDATE sources SET credential_ref = NULL WHERE kind = 'webdav';
            ");
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
        let base = reqwest::Url::parse(base_url)
            .map_err(|e| format!("无效的 WebDAV URL: {}", e))?;
        // 构建完整路径
        let joined = base.join(&format!("{}/{}", remote_dir, filename))
            .map_err(|e| format!("路径组合失败: {}", e))?;
        Ok(joined.to_string())
    }

    /// 上传同步快照：VACUUM INTO → PUT 到 remote_path/lumo.sqlite
    pub fn sync_upload(conn: &Connection, app_dir: &Path, config: &SyncConfigDTO) -> Result<SyncResult, String> {
        let client = Self::build_client(config)?;
        let snapshot = Self::create_snapshot(conn, app_dir)?;
        let file_size = std::fs::metadata(&snapshot).map_err(|e| e.to_string())?.len();
        let data = std::fs::read(&snapshot).map_err(|e| format!("读取快照失败: {}", e))?;

        // 确保远程目录存在
        let remote_dir = config.remote_path.as_deref().unwrap_or("/");
        if remote_dir != "/" && !remote_dir.is_empty() {
            let dir_url = Self::remote_file_url(config, "")?;
            let _ = client.mkcol(&dir_url); // 忽略 405 已存在
        }

        let upload_url = Self::remote_file_url(config, "lumo.sqlite")?;
        client.put_file(&upload_url, data)?;

        // 清理本地快照
        let _ = std::fs::remove_file(&snapshot);

        use chrono::Utc;
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 更新 last_sync_at
        conn.execute(
            "UPDATE sync_config SET last_sync_at = ?1, last_sync_direction = 'upload' WHERE id = 1",
            params![timestamp],
        ).map_err(|e| format!("更新同步时间失败: {}", e))?;

        Ok(SyncResult {
            bytes_uploaded: file_size,
            timestamp,
        })
    }

    /// 下载云端快照到本地临时文件，返回文件路径。
    /// 调用方负责替换 DB 并热重载。
    pub fn sync_download_to_temp(app_dir: &Path, config: &SyncConfigDTO) -> Result<PathBuf, String> {
        let client = Self::build_client(config)?;
        let download_url = Self::remote_file_url(config, "lumo.sqlite")?;
        let temp_path = app_dir.join("lumo_sync_remote.sqlite");
        client.download_to_file(&download_url, &temp_path)?;
        Ok(temp_path)
    }

    /// 检查云端是否有同步数据（用于首次配置检测）
    pub fn check_remote(config: &SyncConfigDTO) -> Result<crate::models::RemoteCheckResult, String> {
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
            if f.path.ends_with("lumo.sqlite") {
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
        Ok(files.into_iter()
            .filter(|f| f.is_dir)
            .collect())
    }

    /// 在 WebDAV 上创建目录
    pub fn create_remote_folder(config: &SyncConfigDTO, path: &str) -> Result<(), String> {
        let client = Self::build_client(config)?;
        client.mkcol(path)
    }
}

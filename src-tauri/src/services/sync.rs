use crate::models::{SyncConfigDTO, SyncResult};
use crate::services::webdav::{WebdavClient, WebdavFile};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

/// 跨设备同步专用密钥（不依赖机器路径，所有设备相同）。
/// 用固定 app identifier 作为种子，确保同一份配置在任意设备上都能解密。
fn derive_sync_key() -> [u8; 32] {
    let seed = b"com.hao.lumo.sync";
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = seed[i % seed.len()];
    }
    key
}

/// XOR + base64 加密（与 scanner.rs 的 encrypt_password 同算法，但密钥不同）
fn encrypt_sync_password(password: &str) -> String {
    let key = derive_sync_key();
    let bytes: Vec<u8> = password.bytes()
        .enumerate()
        .map(|(i, b)| b ^ key[i % 32])
        .collect();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

/// 解密同步密码
fn decrypt_sync_password(encoded: &str) -> Option<String> {
    let key = derive_sync_key();
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).ok()?;
    let decrypted: Vec<u8> = bytes.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % 32])
        .collect();
    String::from_utf8(decrypted).ok()
}

/// 同步服务：管理 sync_config 的 CRUD 及 WebDAV 上传/下载/浏览。
pub struct SyncService;

impl SyncService {
    /// 读取同步配置（密码解密后返回）
    pub fn get_config(conn: &Connection) -> rusqlite::Result<SyncConfigDTO> {
        let row = conn.query_row(
            "SELECT enabled, webdav_url, username, password_encrypted, remote_path, last_sync_at, last_sync_direction
             FROM sync_config WHERE id = 1",
            [],
            |row| {
                Ok(SyncConfigDTO {
                    enabled: row.get(0)?,
                    webdav_url: row.get(1)?,
                    username: row.get(2)?,
                    password: row.get::<_, Option<String>>(3)?.and_then(|e| decrypt_sync_password(&e)),
                    remote_path: row.get(4)?,
                    last_sync_at: row.get(5)?,
                    last_sync_direction: row.get(6)?,
                })
            },
        )?;
        Ok(row)
    }

    /// 保存同步配置（密码加密后写入）
    pub fn save_config(conn: &Connection, config: &SyncConfigDTO) -> rusqlite::Result<()> {
        let password_encrypted = config.password.as_deref().map(encrypt_sync_password);
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

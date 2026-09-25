use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::services::scanner::{finish_scan, ScanOutcome};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use tauri::{Manager, State};

/// 全局扫描中集合：同一来源同一时刻只允许一个扫描任务（P1-10 幂等守卫）。
/// 用全局 static 是因为扫描线程拿不到 Tauri State，需要在任意线程标记/解除。
static SCANNING_SOURCES: LazyLock<Mutex<HashSet<i64>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// RAII 守卫：占用某来源的扫描权，Drop 时解除标记。
///
/// 关键点是**在 `source_scan` 取得数据库连接之前**就构造，并随闭包移入扫描线程。
/// 若在 mark 与守卫构造之间存在 `?` 早退（取连接失败、来源已被删除），
/// 该来源会永久停在"扫描中"，之后每次点击都被幂等守卫拒绝，只能重启进程。
struct ScanGuard(i64);

impl ScanGuard {
    /// 抢占扫描权；该来源已有扫描在进行时返回 None。
    fn try_new(source_id: i64) -> Option<Self> {
        SCANNING_SOURCES
            .lock()
            .map(|mut set| set.insert(source_id))
            .unwrap_or(false)
            .then_some(Self(source_id))
    }
}

impl Drop for ScanGuard {
    fn drop(&mut self) {
        if let Ok(mut set) = SCANNING_SOURCES.lock() {
            set.remove(&self.0);
        }
    }
}

/// 从 app_data_dir 路径推导加密密钥（同一台机器稳定，跨机器不同）。
pub(crate) fn derive_credential_key(app_dir: &std::path::Path) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let secret_file = app_dir.join(".device_secret");
    let device_entropy = if secret_file.exists() {
        std::fs::read(&secret_file).unwrap_or_default()
    } else {
        use rand::Rng;
        let mut buf = [0u8; 32];
        rand::rng().fill_bytes(&mut buf);
        let _ = std::fs::write(&secret_file, buf);
        buf.to_vec()
    };
    if !device_entropy.is_empty() {
        hasher.update(&device_entropy);
    } else {
        hasher.update(app_dir.to_string_lossy().as_bytes());
    }
    hasher.update(b"lumo_credential_secret_v2_seed");
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// 对密码做加密（防止明文暴露，结合设备专属随机私钥）。
pub(crate) fn encrypt_password(key: &[u8; 32], password: &str) -> String {
    use base64::Engine;
    let bytes: Vec<u8> = password
        .bytes()
        .enumerate()
        .map(|(i, b)| b ^ key[i % 32])
        .collect();
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    format!("v2:seal:{}", b64)
}

/// 解密密码（优先解析 v2:seal 规范格式，向下兼容旧数据）。
pub(crate) fn decrypt_password(key: &[u8; 32], encoded: &str) -> Option<String> {
    use base64::Engine;
    if let Some(payload) = encoded.strip_prefix("v2:seal:") {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .ok()?;
        let decrypted: Vec<u8> = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ key[i % 32])
            .collect();
        return String::from_utf8(decrypted).ok();
    }
    // 旧格式兼容
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let decrypted: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % 32])
        .collect();
    String::from_utf8(decrypted).ok()
}

/// 解析来源凭据引用，返回 (username, Option<password>)。统一所有格式的分发入口（P1-08）：
/// - `user##kr:<uuid>`：密码在系统钥匙串（桌面端）。Android 无钥匙串，视为失效。
/// - `user##v2:seal:…`：机器绑定 XOR 加密（V6）。桌面端解析成功后懒迁移进钥匙串。
/// - `user:password`：V5 明文（仅读兼容），桌面端同样懒迁移。
/// - `user`：仅用户名，无密码。
///
/// 钥匙串缺失 / 解密失败返回 NEEDS_REAUTH 语义错误（提示重新添加来源）。
pub(crate) fn resolve_source_credential(
    conn: &rusqlite::Connection,
    source_id: i64,
    credential_ref: &str,
    machine_key: &[u8; 32],
) -> Result<(String, Option<String>), AppError> {
    const NEEDS_REAUTH: &str = "该来源的密码凭据已失效，请删除后重新添加该来源";

    // 1. 系统钥匙串引用（桌面端新格式）
    if let Some((u, entry)) = credential_ref.split_once("##kr:") {
        #[cfg(not(target_os = "android"))]
        {
            match crate::services::secret::keyring_get(entry) {
                Ok(pw) => return Ok((u.to_string(), Some(pw))),
                Err(e) => return Err(AppError::Internal(format!("{} ({})", NEEDS_REAUTH, e))),
            }
        }
        #[cfg(target_os = "android")]
        {
            let _ = u;
            return Err(AppError::Internal(NEEDS_REAUTH.to_string()));
        }
    }

    // 2. user##<encrypted>（V6 加密 / 更旧 base64）
    if let Some((u, enc)) = credential_ref.split_once("##") {
        let Some(pw) = decrypt_password(machine_key, enc) else {
            return Err(AppError::Internal(NEEDS_REAUTH.to_string()));
        };
        lazy_migrate_to_keyring(conn, source_id, u, &pw);
        return Ok((u.to_string(), Some(pw)));
    }

    // 3. user:password（V5 明文）
    if let Some((u, p)) = credential_ref.split_once(':') {
        lazy_migrate_to_keyring(conn, source_id, u, p);
        return Ok((u.to_string(), Some(p.to_string())));
    }

    // 4. 仅用户名
    Ok((credential_ref.to_string(), None))
}

/// WebDAV 扫描开工前的凭据准备（CR-006）。
///
/// 把「取不到数据库连接」和「凭据解析失败」两种早退统一翻译成 [`ScanOutcome`]：
/// 调用方只负责交给 `finish_scan` 落库 + 发事件，两条路径因此共享同一套终态口径。
/// 拆成不依赖 `AppHandle` 的纯逻辑，是为了让这两条失败路径能被单元测试直接驱动
/// ——构造一个 AppHandle 不在测试能力范围内，而构造一个连接池在范围内。
fn prepare_webdav_credentials(
    pool: &crate::db::DbPool,
    source_id: i64,
    credential_ref: Option<&str>,
    machine_key: &[u8; 32],
) -> Result<(Option<String>, Option<String>), ScanOutcome> {
    let Some(cred) = credential_ref else {
        // 无凭据来源（公开共享目录 / 匿名访问）本就是合法配置
        return Ok((None, None));
    };
    let conn = pool.get().map_err(|e| {
        ScanOutcome::failed(
            "db_unavailable",
            format!("扫描无法开始：本地数据库正忙，请稍后重试（{}）", e),
        )
    })?;
    match resolve_source_credential(&conn, source_id, cred, machine_key) {
        Ok((u, p)) => Ok((Some(u), p)),
        Err(e) => {
            // AppError 的 Display 带 "Internal error:" 前缀；last_error 是直接展示给用户的，取内层文案
            let detail = match &e {
                AppError::Internal(msg) => msg.clone(),
                other => other.to_string(),
            };
            Err(ScanOutcome::failed(
                "credential_unresolved",
                format!("凭据解析失败：{}", detail),
            ))
        }
    }
}

/// 桌面端把解析出的明文密码升级进系统钥匙串，并把 credential_ref 改写为 kr 引用。
/// best-effort：钥匙串写入失败（如无可用后端）保留原格式，不影响功能。
fn lazy_migrate_to_keyring(
    conn: &rusqlite::Connection,
    source_id: i64,
    username: &str,
    password: &str,
) {
    #[cfg(not(target_os = "android"))]
    {
        let uuid = hex::encode(rand::random::<[u8; 16]>());
        if crate::services::secret::keyring_set(&uuid, password).is_ok() {
            let _ = conn.execute(
                "UPDATE sources SET credential_ref = ?1 WHERE id = ?2",
                rusqlite::params![format!("{}##kr:{}", username, uuid), source_id],
            );
        }
    }
    #[cfg(target_os = "android")]
    {
        let _ = (conn, source_id, username, password);
    }
}

/// 从 credential_ref 解析出用户名（供前端展示；密码部分永不透出）。
pub(crate) fn username_from_credential_ref(cred: Option<&String>) -> Option<String> {
    cred.and_then(|c| {
        c.split_once("##")
            .map(|(u, _)| u.to_string())
            .or_else(|| c.split_once(':').map(|(u, _)| u.to_string()))
            .or_else(|| Some(c.clone()))
    })
    .filter(|s| !s.is_empty())
}

#[tauri::command]
pub fn source_add_local(
    db_state: State<'_, DbState>,
    path: String,
    name: String,
) -> Result<i64, AppError> {
    let _trace = ipc_trace!("source_add_local");
    let conn = db_state.db.get()?;
    conn.execute(
        "INSERT INTO sources (name, kind, root_uri) VALUES (?1, 'local', ?2)",
        rusqlite::params![name, path],
    )?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

#[tauri::command]
pub fn source_add_webdav(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
    url: String,
    name: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<i64, AppError> {
    let _trace = ipc_trace!("source_add_webdav");
    let conn = db_state.db.get()?;

    // Test connection (use empty subpath so PROPFIND hits the exact base URL, not the server root)
    let webdav =
        // 联网行为: §2E —— 地址来自 sources.root_uri（用户自填）
        crate::services::webdav::WebdavClient::new(url.clone(), username.clone(), password.clone());
    webdav
        .propfind("")
        .map_err(|e| AppError::Internal(format!("Failed to connect to WebDAV: {}", e)))?;

    // credential_ref 格式（P1-08）：桌面端优先存系统钥匙串引用 "username##kr:<uuid>"；
    // 钥匙串不可用时回退机器绑定加密 "username##v2:seal:…"。Android 始终用机器绑定加密。
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let key = derive_credential_key(&app_dir);
    let cred = match (&username, &password) {
        (Some(u), Some(p)) => {
            #[cfg(not(target_os = "android"))]
            {
                let uuid = hex::encode(rand::random::<[u8; 16]>());
                match crate::services::secret::keyring_set(&uuid, p) {
                    Ok(()) => Some(format!("{}##kr:{}", u, uuid)),
                    Err(e) => {
                        tracing::warn!("钥匙串写入失败，回退机器绑定加密: {}", e);
                        Some(format!("{}##{}", u, encrypt_password(&key, p)))
                    }
                }
            }
            #[cfg(target_os = "android")]
            {
                Some(format!("{}##{}", u, encrypt_password(&key, p)))
            }
        }
        (Some(u), None) => Some(u.clone()),
        (None, _) => None,
    };

    conn.execute(
        "INSERT INTO sources (name, kind, root_uri, credential_ref) VALUES (?1, 'webdav', ?2, ?3)",
        rusqlite::params![name, url, cred],
    )?;

    Ok(conn.last_insert_rowid())
}

/// [MA3 A3-1] 测试 WebDAV 连接（添加来源前即时反馈连通性与权限）
#[tauri::command]
pub fn scanner_test_webdav(
    url: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<crate::services::webdav::WebdavProbeResult, AppError> {
    let _trace = ipc_trace!("scanner_test_webdav");
    // 联网行为: §2E
    let webdav = crate::services::webdav::WebdavClient::new(url, username, password);
    Ok(webdav.probe_connection())
}

#[tauri::command]
pub fn source_scan(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
    source_id: i64,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("source_scan");
    // P1-10 幂等守卫：同一来源重复触发扫描直接拒绝（前端 UI 有软守卫，这里是硬防线）。
    // 守卫必须在下面任何 `?` 之前取得，早退时才会自动释放扫描权。
    let scan_guard = ScanGuard::try_new(source_id)
        .ok_or_else(|| AppError::Internal("该来源正在扫描中，请等待本次扫描完成".to_string()))?;
    let (kind, path, credential) = {
        let conn = db_state.db.get()?;
        let (k, r, c): (String, String, Option<String>) = conn.query_row(
            "SELECT kind, root_uri, credential_ref FROM sources WHERE id = ?1",
            rusqlite::params![source_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        (k, r, c)
    };

    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let key = derive_credential_key(&app_dir);

    let _handle = std::thread::Builder::new()
        .name(format!("lumo-scan-{}", source_id))
        .spawn(move || {
            // 守卫随闭包进入线程：正常结束 / 提前 return / panic 都会解除标记
            let _scan_guard = scan_guard;
            if kind == "local" {
                crate::services::scanner::scan_local_directory(
                    app,
                    source_id,
                    &PathBuf::from(path),
                    &app_dir,
                );
            } else if kind == "webdav" {
                // 凭据解析（P1-08 统一入口）：支持钥匙串引用 / V6 加密 / V5 明文；
                // 桌面端解析成功后懒迁移进系统钥匙串。
                // 失败必须带原因地终止（CR-006）：只 emit 一个 source_id 会让前端
                // 回读到上一次的旧状态，用户看不出这次扫描到底为什么没成。
                let prepared = match app.try_state::<DbState>() {
                    Some(db) => {
                        prepare_webdav_credentials(&db.db, source_id, credential.as_deref(), &key)
                    }
                    None => Err(ScanOutcome::failed(
                        "db_unavailable",
                        "扫描无法开始：本地数据库不可用，请重启应用后重试",
                    )),
                };
                let (username, password) = match prepared {
                    Ok(pair) => pair,
                    Err(outcome) => {
                        finish_scan(&app, source_id, outcome);
                        return;
                    }
                };
                crate::services::scanner::scan_webdav_directory(
                    app, source_id, path, username, password, &app_dir,
                );
            }
        })
        .map_err(|e| {
            // 起不了线程时闭包被丢弃，守卫随之释放；必须把失败告知用户，
            // 否则前端以为扫描已启动，进度条会一直挂着。
            AppError::Internal(format!("无法启动扫描任务: {}", e))
        })?;

    Ok(())
}

#[tauri::command]
pub fn source_list(db_state: State<'_, DbState>) -> Result<Vec<crate::models::Source>, AppError> {
    let _trace = ipc_trace!("source_list");
    let conn = db_state.db.get()?;
    let mut stmt = conn.prepare("
        SELECT id, name, kind, root_uri, config_json, credential_ref, enabled, last_scan_at, last_error, created_at, updated_at 
        FROM sources 
        ORDER BY created_at DESC
    ")?;

    let sources = stmt
        .query_map([], |row| {
            let credential_ref: Option<String> = row.get(5)?;
            let username = username_from_credential_ref(credential_ref.as_ref());
            Ok(crate::models::Source {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                root_uri: row.get(3)?,
                config_json: row.get(4)?,
                credential_ref,
                username,
                enabled: row.get::<_, i64>(6)? != 0,
                last_scan_at: row.get(7)?,
                last_error: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(sources)
}

#[tauri::command]
pub fn source_remove(db_state: State<'_, DbState>, source_id: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("source_remove");
    let mut conn = db_state.db.get()?;

    // 删除前清理该来源的系统钥匙串条目（kr 引用格式），避免孤儿凭据残留（P1-08）
    if let Ok(Some(cred)) = conn.query_row(
        "SELECT credential_ref FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| row.get::<_, Option<String>>(0),
    ) {
        if let Some((_, entry)) = cred.split_once("##kr:") {
            // kr 引用只在桌面端产生（Android 无钥匙串，见 resolve_source_credential 注释），
            // keyring_delete 仅桌面编译；Android 走空分支。
            #[cfg(not(target_os = "android"))]
            crate::services::secret::keyring_delete(entry);
            #[cfg(target_os = "android")]
            let _ = entry;
        }
    }

    let tx = conn.transaction()?;

    tx.execute(
        "DELETE FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
    )?;

    // primary_file_id is intentionally not a hard foreign key because it is a
    // denormalised preference. Re-point it before orphan cleanup so removing one
    // source cannot hide a track that still has another available file.
    // 重指时按文件质量分（本地 > 远程、无损 > 有损）选择，与扫描期选主逻辑一致。
    tx.execute(
        "UPDATE tracks
         SET primary_file_id = (
             SELECT mf.id FROM media_files mf
             JOIN sources s ON s.id = mf.source_id
             WHERE mf.track_id = tracks.id AND mf.availability = 'available'
             ORDER BY (CASE s.kind WHEN 'local' THEN 100 ELSE 0 END)
                    + (CASE lower(mf.file_ext)
                        WHEN 'flac' THEN 50 WHEN 'wav' THEN 50 WHEN 'm4a' THEN 30 WHEN 'aac' THEN 30
                        WHEN 'mp3' THEN 10 WHEN 'ogg' THEN 10 ELSE 0 END)
                     DESC, mf.id
             LIMIT 1
         )
         WHERE primary_file_id IS NULL
            OR NOT EXISTS (
                SELECT 1 FROM media_files mf
                WHERE mf.id = tracks.primary_file_id AND mf.availability = 'available'
            )",
        [],
    )?;

    tx.execute(
        "DELETE FROM tracks WHERE id NOT IN (SELECT track_id FROM media_files WHERE track_id IS NOT NULL)",
        [],
    )?;

    tx.execute(
        "DELETE FROM albums WHERE id NOT IN (SELECT album_id FROM tracks WHERE album_id IS NOT NULL)",
        [],
    )?;

    tx.execute(
        "DELETE FROM artists WHERE id NOT IN (
            SELECT album_artist_id FROM albums WHERE album_artist_id IS NOT NULL
            UNION
            SELECT artist_id FROM track_artists
        )",
        [],
    )?;

    // 清理无主的 artwork 记录（对应 P1-4）
    // artwork 通过 media_file_id 关联 media_files，media_files 已通过 ON DELETE CASCADE 被删除，
    // 但 artwork.media_file_id 是 SET NULL，需要主动清理引用计数为 0 的 artwork。
    {
        let orphan_artworks: Vec<(i64, Option<String>)> = {
            let mut stmt = tx.prepare(
                "SELECT a.id, a.cache_path FROM artwork a
                 WHERE a.id NOT IN (
                     SELECT cover_artwork_id FROM albums WHERE cover_artwork_id IS NOT NULL
                 )",
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            let collected: Vec<(i64, Option<String>)> = rows.filter_map(|r| r.ok()).collect();
            collected
        };

        for (art_id, cache_path) in &orphan_artworks {
            tx.execute(
                "DELETE FROM artwork WHERE id = ?1",
                rusqlite::params![art_id],
            )?;
            if let Some(path) = cache_path {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    tx.execute_batch(
        "UPDATE albums SET track_count = (
            SELECT COUNT(*) FROM tracks t WHERE t.album_id = albums.id
        );
        UPDATE artists SET track_count = (
            SELECT COUNT(DISTINCT ta.track_id) FROM track_artists ta WHERE ta.artist_id = artists.id
        );
        UPDATE artists SET album_count = (
            SELECT COUNT(*) FROM albums al WHERE al.album_artist_id = artists.id
        );",
    )?;

    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_util::TempDir;
    use std::time::Duration;

    /// 建一份带单个 WebDAV 来源的测试库，返回 (临时目录, 连接池, source_id)。
    /// 凭据引用直接写进 `sources.credential_ref`，模拟"上一次保存成功、这次解不开"的真实状态。
    fn pool_with_source(
        label: &str,
        credential_ref: Option<&str>,
    ) -> (TempDir, crate::db::DbPool, i64) {
        let dir = TempDir::new(label);
        let pool = crate::db::init_db(dir.db_path()).expect("构造测试库失败");
        let conn = pool.get().expect("取连接失败");
        conn.execute(
            "INSERT INTO sources (name, kind, root_uri, credential_ref) VALUES ('来源', 'webdav', 'http://127.0.0.1/dav', ?1)",
            rusqlite::params![credential_ref],
        )
        .expect("插入来源失败");
        let id = conn.last_insert_rowid();
        (dir, pool, id)
    }

    /// 匿名共享目录没有凭据引用，本就不是失败：不能因为 cred 为空而中止扫描。
    #[test]
    fn source_without_credential_is_not_a_failure() {
        let (_dir, pool, id) = pool_with_source("cred_none", None);
        let (user, pass) = prepare_webdav_credentials(&pool, id, None, &[0u8; 32])
            .expect("无凭据来源应允许匿名扫描");
        assert_eq!(user, None);
        assert_eq!(pass, None);
    }

    /// 凭据失效（CR-006 的核心场景）：早退必须变成带原因的终态，而不是只发一条完成事件。
    /// 断言文案可直接展示给用户，且不泄露任何口令内容。
    #[test]
    fn expired_credential_becomes_a_terminal_outcome() {
        let stored = "alice##乱码不是合法密文";
        let (_dir, pool, id) = pool_with_source("cred_expired", Some(stored));
        let err = prepare_webdav_credentials(&pool, id, Some(stored), &[0u8; 32])
            .expect_err("解不开的凭据必须中止扫描");
        let ScanOutcome::Failed { code, message } = err else {
            panic!("凭据失效必须是 Failed 终态，得到 {:?}", err)
        };
        assert_eq!(code, "credential_unresolved");
        assert!(
            message.contains("凭据已失效"),
            "文案要说明该怎么做：{}",
            message
        );
        assert!(
            !message.contains("乱码"),
            "不能把凭据引用原文回显给用户：{}",
            message
        );
    }

    /// 连接池耗尽（CR-006 建议 5 的另一半）：拿不到库同样要有明确终态与原因。
    #[test]
    fn exhausted_pool_becomes_a_terminal_outcome() {
        let dir = TempDir::new("cred_pool_busy");
        crate::db::init_db(dir.db_path()).expect("建库失败");
        // 只给一条连接并先占住，第二次 get 在超时后失败——就是线上"数据库正忙"的形态
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .connection_timeout(Duration::from_millis(500))
            .build(r2d2_sqlite::SqliteConnectionManager::file(dir.db_path()))
            .expect("构造连接池失败");
        let held = pool.get().expect("首次取连接应成功");

        let err = prepare_webdav_credentials(&pool, 1, Some("alice"), &[0u8; 32])
            .expect_err("池被占满时不能假装扫描已启动");
        let ScanOutcome::Failed { code, message } = err else {
            panic!("取不到连接必须是 Failed 终态，得到 {:?}", err)
        };
        assert_eq!(code, "db_unavailable");
        assert!(
            message.contains("数据库正忙"),
            "文案要让用户知道稍后重试：{}",
            message
        );
        drop(held);
    }
}

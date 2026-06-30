use tauri::{State, Manager};
use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use std::path::PathBuf;

/// 从 app_data_dir 路径推导加密密钥（同一台机器稳定，跨机器不同）。
pub(crate) fn derive_credential_key(app_dir: &std::path::Path) -> [u8; 32] {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    app_dir.to_string_lossy().hash(&mut hasher);
    let seed = hasher.finish().to_le_bytes();
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = seed[i % 8];
    }
    key
}

/// 对密码做 XOR + base64 编码（防止明文暴露，机器绑定）。
pub(crate) fn encrypt_password(key: &[u8; 32], password: &str) -> String {
    let bytes: Vec<u8> = password.bytes()
        .enumerate()
        .map(|(i, b)| b ^ key[i % 32])
        .collect();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

/// 解密用 encrypt_password 编码的密码。
pub(crate) fn decrypt_password(key: &[u8; 32], encoded: &str) -> Option<String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).ok()?;
    let decrypted: Vec<u8> = bytes.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % 32])
        .collect();
    String::from_utf8(decrypted).ok()
}

#[tauri::command]
pub fn source_add_local(db_state: State<'_, DbState>, path: String, name: String) -> Result<i64, AppError> {
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
pub fn source_add_webdav(app: tauri::AppHandle, db_state: State<'_, DbState>, url: String, name: String, username: Option<String>, password: Option<String>) -> Result<i64, AppError> {
    let _trace = ipc_trace!("source_add_webdav");
    let conn = db_state.db.get()?;

    // Test connection
    let webdav = crate::services::webdav::WebdavClient::new(url.clone(), username.clone(), password.clone());
    webdav.propfind("/").map_err(|e| AppError::Internal(format!("Failed to connect to WebDAV: {}", e)))?;

    // credential_ref 格式：新来源存为 "username##base64_encrypted_password"
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let key = derive_credential_key(&app_dir);
    let cred = match (&username, &password) {
        (Some(u), Some(p)) => Some(format!("{}##{}", u, encrypt_password(&key, p))),
        (Some(u), None) => Some(u.clone()),
        (None, _) => None,
    };

    conn.execute(
        "INSERT INTO sources (name, kind, root_uri, credential_ref) VALUES (?1, 'webdav', ?2, ?3)",
        rusqlite::params![name, url, cred],
    )?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn source_scan(app: tauri::AppHandle, db_state: State<'_, DbState>, source_id: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("source_scan");
    let (kind, path, credential) = {
        let conn = db_state.db.get()?;
        let (k, r, c): (String, String, Option<String>) = conn.query_row(
            "SELECT kind, root_uri, credential_ref FROM sources WHERE id = ?1",
            rusqlite::params![source_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        (k, r, c)
    };

    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let key = derive_credential_key(&app_dir);

    std::thread::spawn(move || {
        if kind == "local" {
            crate::services::scanner::scan_local_directory(app, source_id, &PathBuf::from(path), &app_dir);
        } else if kind == "webdav" {
            // 从 credential_ref 提取用户名和密码，兼容三种格式：
            // 1) "username##base64_encrypted" ─ V6+ 加密格式
            // 2) "username:password" ─ V5 及之前明文（迁移/旧库兼容）
            // 3) "username" ─ 仅有用户名（无密码认证）
            let (username, password) = credential.as_deref()
                .and_then(|cred| {
                    if let Some((u, enc)) = cred.split_once("##") {
                        decrypt_password(&key, enc).map(|p| (u.to_string(), p))
                    } else if let Some((u, p)) = cred.split_once(':') {
                        Some((u.to_string(), p.to_string()))
                    } else {
                        Some((cred.to_string(), String::new()))
                    }
                })
                .map(|(u, p)| (Some(u), Some(p)))
                .unwrap_or((None, None));
            crate::services::scanner::scan_webdav_directory(app, source_id, path, username, password, &app_dir);
        }
    });
    
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
    
    let sources = stmt.query_map([], |row| {
        Ok(crate::models::Source {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            root_uri: row.get(3)?,
            config_json: row.get(4)?,
            credential_ref: row.get(5)?,
            enabled: row.get::<_, i64>(6)? != 0,
            last_scan_at: row.get(7)?,
            last_error: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    
    Ok(sources)
}

#[tauri::command]
pub fn source_remove(db_state: State<'_, DbState>, source_id: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("source_remove");
    let mut conn = db_state.db.get()?;

    let tx = conn.transaction()?;

    tx.execute("DELETE FROM sources WHERE id = ?1", rusqlite::params![source_id])?;

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
                 )"
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            let collected: Vec<(i64, Option<String>)> = rows.filter_map(|r| r.ok()).collect();
            collected
        };

        for (art_id, cache_path) in &orphan_artworks {
            tx.execute("DELETE FROM artwork WHERE id = ?1", rusqlite::params![art_id])?;
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
        );"
    )?;

    tx.commit()?;
    Ok(())
}

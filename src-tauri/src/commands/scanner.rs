use tauri::{State, Manager};
use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use std::path::PathBuf;

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
pub fn source_add_webdav(db_state: State<'_, DbState>, url: String, name: String, username: Option<String>, password: Option<String>) -> Result<i64, AppError> {
    let _trace = ipc_trace!("source_add_webdav");
    let conn = db_state.db.get()?;
    
    // Test connection
    let webdav = crate::services::webdav::WebdavClient::new(url.clone(), username.clone(), password.clone());
    webdav.propfind("/").map_err(|e| AppError::Internal(format!("Failed to connect to WebDAV: {}", e)))?;

    let cred = if let (Some(u), Some(p)) = (&username, &password) {
        Some(format!("{}:{}", u, p))
    } else if let Some(u) = &username {
        Some(u.clone())
    } else {
        None
    };

    conn.execute(
        "INSERT INTO sources (name, kind, root_uri, credential_ref) VALUES (?1, 'webdav', ?2, ?3)",
        rusqlite::params![name, url, cred],
    )?;
    
    let id = conn.last_insert_rowid();
    Ok(id)
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
    
    std::thread::spawn(move || {
        if kind == "local" {
            crate::services::scanner::scan_local_directory(app, source_id, &PathBuf::from(path), &app_dir);
        } else if kind == "webdav" {
            let mut username = None;
            let mut password = None;
            if let Some(cred) = credential {
                let parts: Vec<&str> = cred.splitn(2, ':').collect();
                if parts.len() == 2 {
                    username = Some(parts[0].to_string());
                    password = Some(parts[1].to_string());
                } else if parts.len() == 1 {
                    username = Some(parts[0].to_string());
                }
            }
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

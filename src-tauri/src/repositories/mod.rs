pub mod album_repo;
pub mod artist_repo;
pub mod playlist_repo;
pub mod track_repo;

use crate::models::TrackDTO;
use base64::{engine::general_purpose, Engine as _};

/// 把 artwork.thumbnail_blob 转成前端可直接渲染的 base64 data URL。
/// 与专辑/艺人网格共用同一约定：列表 IPC 内联缩略图，前端不再逐个发 lumo://artwork 请求。
pub fn thumbnail_to_data_url(blob: Option<Vec<u8>>) -> Option<String> {
    blob.map(|b| {
        format!(
            "data:image/jpeg;base64,{}",
            general_purpose::STANDARD.encode(&b)
        )
    })
}

/// 转义 SQLite LIKE 模式串中的特殊字符（`%` / `_` / `\`），避免路径里这些字符被当通配符。
/// 转义符是反斜杠，调用方需配合 `LIKE ? ESCAPE '\'` 使用（见 playlist_repo 的用法）。
pub fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

pub fn map_track_row(row: &rusqlite::Row) -> rusqlite::Result<TrackDTO> {
    Ok(TrackDTO {
        id: row.get(0)?,
        title: row.get(1)?,
        artist_id: row.get(2)?,
        artist_name: row.get(3)?,
        album_id: row.get(4)?,
        album_title: row.get(5)?,
        duration_ms: row.get(6)?,
        format: row.get(7)?,
        media_file_id: row.get(8)?,
        is_favorite: row.get(9)?,
        cover_artwork_id: row.get(10)?,
        last_played_at: None,
        file_size: row.get::<_, Option<i64>>(11)?,
        source_kind: row.get::<_, String>(12)?,
    })
}

pub mod track_repo;
pub mod album_repo;
pub mod artist_repo;
pub mod playlist_repo;

use crate::models::TrackDTO;
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

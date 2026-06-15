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
            artist_name: row.get(2)?,
            album_title: row.get(3)?,
            duration_ms: row.get(4)?,
            format: row.get(5)?,
            media_file_id: row.get(6)?,
            is_favorite: row.get(7)?,
            cover_artwork_id: row.get(8)?,
        })
    }

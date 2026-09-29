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

/// 列表查询共用的 TrackDTO 扩展列（LDL v2 丰富歌曲信息）。
///
/// 追加位置与顺序和 map_track_row 的读取索引强耦合：
/// - 13 列标准查询：跟在 source_kind(12) 之后 → year(13) genres(14) bitrate(15) sample_rate(16) bit_depth(17)
/// - 最近播放（14 列）：跟在 last_played_at(13) 之后 → 14~18
/// - 排行榜详情（15 列）：跟在 last_played_at(14) 之后 → 15~19
///
/// 语义约定：
/// - year = 歌曲年份优先、专辑发行年份兜底（可空）；
/// - genres 用聚合子查询（JOIN 一对多会把分页行数撑爆），多流派以 "; " 连接；
/// - bitrate/sample_rate/bit_depth 描述**当前首选媒体文件 m**（跟随多音源切换）。
pub const TRACK_EXTRA_COLUMNS_SQL: &str = "\n        COALESCE(t.year, al.release_year),\n        (SELECT GROUP_CONCAT(g.name, '; ') FROM track_genres tg JOIN genres g ON g.id = tg.genre_id WHERE tg.track_id = t.id),\n        m.bitrate,\n        m.sample_rate,\n        m.bit_depth";

/// 标准 13 列查询在 source_kind 行尾追加该片段即可（`AS source_kind,` + 本片段）。
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
        year: row.get::<_, Option<i64>>(13)?,
        genres: row.get::<_, Option<String>>(14)?,
        bitrate: row.get::<_, Option<i64>>(15)?,
        sample_rate: row.get::<_, Option<i64>>(16)?,
        bit_depth: row.get::<_, Option<i64>>(17)?,
    })
}

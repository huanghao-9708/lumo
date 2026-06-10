use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub root_uri: String,
    pub config_json: String,
    pub credential_ref: Option<String>,
    pub enabled: bool,
    pub last_scan_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub normalized_name: String,
    pub sort_name: Option<String>,
    pub kind: String,
    pub mbid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub normalized_title: String,
    pub sort_title: Option<String>,
    pub album_artist_id: Option<i64>,
    pub album_type: String,
    pub release_date: Option<String>,
    pub release_year: Option<i64>,
    pub total_discs: Option<i64>,
    pub cover_artwork_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub normalized_title: String,
    pub sort_title: Option<String>,
    pub album_id: Option<i64>,
    pub disc_no: Option<i64>,
    pub track_no: Option<i64>,
    pub year: Option<i64>,
    pub primary_file_id: Option<i64>,
    pub rating: Option<i64>,
    pub play_count: i64,
    pub skip_count: i64,
    pub last_played_at: Option<String>,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    pub id: i64,
    pub source_id: i64,
    pub track_id: Option<i64>,
    pub relative_path: String,
    pub normalized_path: String,
    pub file_name: String,
    pub file_ext: Option<String>,
    pub file_size: Option<i64>,
    pub modified_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub bitrate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channels: Option<i64>,
    pub availability: String,
}

#[derive(Debug, serde::Serialize)]
pub struct TrackDTO {
    pub id: i64,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub duration_ms: Option<i64>,
    pub format: Option<String>,
    pub media_file_id: i64,
    pub is_favorite: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct AlbumDTO {
    pub id: i64,
    pub title: String,
    pub artist_name: Option<String>,
    pub cover_artwork_id: Option<i64>,
    pub track_count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct ArtistDTO {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct PlaylistDTO {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
}

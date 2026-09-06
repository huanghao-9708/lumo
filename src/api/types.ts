export interface TrackDTO {
  id: number;
  title: string;
  artist_id: number | null;
  artist_name: string | null;
  album_id: number | null;
  album_title: string | null;
  duration_ms: number | null;
  format: string | null;
  media_file_id: number;
  is_favorite: boolean;
  cover_artwork_id?: number | null;
  last_played_at?: string | null;
  file_size: number | null;
  source_kind: string; // "local" | "webdav"
}

export interface AlbumDTO {
  id: number;
  title: string;
  artist_name: string | null;
  cover_artwork_id: number | null;
  track_count: number;
  /** 专辑发行年份 */
  release_year?: number | null;
  /** 200x200 JPEG 缩略图的 base64 data URL（若有）。
   *  后端 library_get_albums 内联返回，前端可直接用作 <img src>，
   *  无需再发 lumo://artwork 请求。仅当扫描期已生成缩略图时才有值。 */
  cover_thumbnail_base64?: string | null;
}

export interface ArtistDTO {
  id: number;
  name: string;
  track_count: number;
  avatar_artwork_id?: number | null;
}

export interface ArtistListResult {
  artists: ArtistDTO[];
  total: number;
}

export interface PlaylistDTOBackend {
  id: number;
  name: string;
  description: string | null;
  track_count: number;
}

export interface FolderContentsResultDTO {
  entries: Array<{
    name: string;
    is_dir: boolean;
    path: string;
    track?: TrackDTO;
  }>;
  total: number;
}

export interface DirectoryNodeDTO {
  name: string;
  path: string;
  audio_count: number;
  has_subdirs: boolean;
}

export interface FolderChildrenResultDTO {
  children: DirectoryNodeDTO[];
  source_root: string;
}

export interface FolderTracksResultDTO {
  tracks: TrackDTO[];
  total: number;
}

export interface LibraryCountsDTO {
  tracks: number;
  favorite_tracks: number;
  favorite_albums: number;
  favorite_artists: number;
  recently_played: number;
}

export interface ArtistStatsDTO {
  track_count: number;
  album_count: number;
}

export interface TrackFileInfoDTO {
  id: number;
  source_id?: number;
  track_id?: number;
  path: string;
  relative_path?: string;
  file_name?: string;
  file_ext?: string | null;
  file_size: number | null;
  modified_at?: string | null;
  duration_ms: number | null;
  bitrate: number | null;
  sample_rate: number | null;
  bit_depth: number | null;
  channels: number | null;
  format: string | null;
  source_kind?: string;
}

export interface SourceDTO {
  id: number;
  name: string;
  kind: string;
  root_uri: string;
  config_json: string;
  /** 来源用户名（后端从凭据引用解出；密码/引用永不透出前端，P1-08） */
  username?: string | null;
  enabled: boolean;
  last_scan_at: string | null;
  last_error: string | null;
  created_at: string;
  updated_at: string;
}

export interface TrackDTO {
  id: number;
  title: string;
  artist_name: string | null;
  album_title: string | null;
  duration_ms: number | null;
  format: string | null;
  media_file_id: number;
  is_favorite: boolean;
  cover_artwork_id?: number | null;
  last_played_at?: string | null;
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

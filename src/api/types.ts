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
}

export interface AlbumDTO {
  id: number;
  title: string;
  artist_name: string | null;
  cover_artwork_id: number | null;
  track_count: number;
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

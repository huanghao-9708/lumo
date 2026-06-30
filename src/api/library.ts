import { invoke } from '../utils/tauriInvoke';
import type {
  TrackDTO,
  AlbumDTO,
  ArtistDTO,
  PlaylistDTOBackend,
  FolderContentsResultDTO,
  FolderChildrenResultDTO,
  FolderTracksResultDTO,
  LibraryCountsDTO,
  ArtistStatsDTO,
  TrackFileInfoDTO
} from './types';

// Tracks
export function libraryGetTracks(limit: number, offset: number, searchKeyword?: string): Promise<TrackDTO[]> {
  return invoke('library_get_tracks', { limit, offset, searchKeyword });
}

export function libraryGetAlbums(limit: number, offset: number, searchKeyword?: string): Promise<AlbumDTO[]> {
  return invoke('library_get_albums', { limit, offset, searchKeyword });
}

export function libraryGetAlbumCount(searchKeyword?: string): Promise<number> {
  return invoke('library_get_album_count', { searchKeyword });
}

export function libraryGetArtists(limit: number, offset: number, searchKeyword?: string): Promise<ArtistDTO[]> {
  return invoke('library_get_artists', { limit, offset, searchKeyword });
}

export function libraryGetAlbumTracks(albumId: number): Promise<TrackDTO[]> {
  return invoke('library_get_album_tracks', { albumId });
}

export function libraryGetArtistAlbums(artistId: number, limit: number, offset: number): Promise<AlbumDTO[]> {
  return invoke('library_get_artist_albums', { artistId, limit, offset });
}

export function libraryGetArtistAlbumCount(artistId: number): Promise<number> {
  return invoke('library_get_artist_album_count', { artistId });
}

export function libraryGetArtistTracks(artistId: number, limit: number, offset: number): Promise<TrackDTO[]> {
  return invoke('library_get_artist_tracks', { artistId, limit, offset });
}

export function libraryGetArtistStats(artistId: number): Promise<ArtistStatsDTO> {
  return invoke('library_get_artist_stats', { artistId });
}

// Playlists
export function libraryCreatePlaylist(name: string, description?: string): Promise<number> {
  return invoke('library_create_playlist', { name, description });
}

export function libraryGetPlaylists(): Promise<PlaylistDTOBackend[]> {
  return invoke('library_get_playlists');
}

export function libraryAddToPlaylist(playlistId: number, trackId: number): Promise<void> {
  return invoke('library_add_to_playlist', { playlistId, trackId });
}

export function libraryGetPlaylistTracks(playlistId: number): Promise<TrackDTO[]> {
  return invoke('library_get_playlist_tracks', { playlistId });
}

export function libraryDeletePlaylist(playlistId: number): Promise<void> {
  return invoke('library_delete_playlist', { playlistId });
}

export function libraryRemovePlaylistItem(playlistId: number, trackId: number): Promise<void> {
  return invoke('library_remove_playlist_item', { playlistId, trackId });
}

export function libraryAddFolderToPlaylist(sourceId: number, folderPath: string, playlistId: number): Promise<void> {
  return invoke('library_add_folder_to_playlist', { sourceId, folderPath, playlistId });
}

// User Actions
export function libraryToggleFavorite(trackId: number, isFavorite: boolean): Promise<void> {
  return invoke('library_toggle_favorite', { trackId, isFavorite });
}

export function libraryRecordPlay(trackId: number, durationMs: number): Promise<void> {
  return invoke('library_record_play', { trackId, durationMs });
}

export function libraryGetRecentlyPlayed(limit: number): Promise<TrackDTO[]> {
  return invoke('library_get_recently_played', { limit });
}

export function libraryGetFavoriteTracks(): Promise<TrackDTO[]> {
  return invoke('library_get_favorite_tracks');
}

export function libraryGetFavoriteAlbums(): Promise<AlbumDTO[]> {
  return invoke('library_get_favorite_albums');
}

export function libraryGetFavoriteArtists(): Promise<ArtistDTO[]> {
  return invoke('library_get_favorite_artists');
}

export function libraryToggleFavoriteAlbum(albumId: number, isFavorite: boolean): Promise<void> {
  return invoke('library_toggle_favorite_album', { albumId, isFavorite });
}

export function libraryToggleFavoriteArtist(artistId: number, isFavorite: boolean): Promise<void> {
  return invoke('library_toggle_favorite_artist', { artistId, isFavorite });
}

export function librarySavePlayQueue(trackIds: number[]): Promise<void> {
  return invoke('library_save_play_queue', { trackIds });
}

export function libraryGetPlayQueue(): Promise<TrackDTO[]> {
  return invoke('library_get_play_queue');
}

// Cache and File System
export function libraryClearCache(): Promise<void> {
  return invoke('library_clear_cache');
}

export function libraryGetFolderContents(sourceId: number, folderPath?: string, limit?: number, offset?: number): Promise<FolderContentsResultDTO> {
  return invoke('library_get_folder_contents', { sourceId, folderPath, limit, offset });
}

export function libraryGetFolderChildren(sourceId: number, folderPath?: string): Promise<FolderChildrenResultDTO> {
  return invoke('library_get_folder_children', { sourceId, folderPath });
}

export function libraryGetFolderTracks(sourceId: number, folderPath: string, limit: number, offset: number): Promise<FolderTracksResultDTO> {
  return invoke('library_get_folder_tracks', { sourceId, folderPath, limit, offset });
}

export function libraryGetCounts(): Promise<LibraryCountsDTO> {
  return invoke('library_get_counts');
}

export function libraryGetLyrics(trackId: number): Promise<string | null> {
  return invoke('library_get_lyrics', { trackId });
}

export function libraryGetTrackFileInfo(trackId: number): Promise<TrackFileInfoDTO | null> {
  return invoke('library_get_track_file_info', { trackId });
}


export function libraryGetCacheSize(): Promise<number> {
  return invoke('library_get_cache_size');
}


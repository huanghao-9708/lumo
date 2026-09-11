import { invoke } from '../utils/tauriInvoke';
import type {
  TrackDTO,
  AlbumDTO,
  ArtistDTO,
  ArtistListResult,
  PlaylistDTOBackend,
  FolderContentsResultDTO,
  FolderChildrenResultDTO,
  FolderTracksResultDTO,
  LibraryCountsDTO,
  ArtistStatsDTO,
  TrackFileInfoDTO,
  LibraryStatsDTO,
  LibraryInsightsDTO
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

export function libraryGetArtists(limit: number, offset: number, searchKeyword?: string): Promise<ArtistListResult> {
  return invoke('library_get_artists', { limit, offset, searchKeyword });
}

export function libraryGetAlbumById(albumId: number): Promise<AlbumDTO | null> {
  return invoke('library_get_album_by_id', { albumId });
}

export function libraryGetArtistById(artistId: number): Promise<ArtistDTO | null> {
  return invoke('library_get_artist_by_id', { artistId });
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

/** 批量添加歌曲到歌单（后端单事务，自动跳过重复）。返回 [成功添加数, 跳过的重复数] */
export function libraryAddTracksToPlaylist(playlistId: number, trackIds: number[]): Promise<[number, number]> {
  return invoke('library_add_tracks_to_playlist', { playlistId, trackIds });
}

/** 批量设置/取消收藏（后端单事务） */
export function librarySetFavoriteBatch(trackIds: number[], isFavorite: boolean): Promise<void> {
  return invoke('library_set_favorite_batch', { trackIds, isFavorite });
}

// Home (stats & insights)
/** 首页统计：曲库规模 + 收听行为，一条 SQL 聚合 */
export function libraryGetStats(): Promise<LibraryStatsDTO> {
  return invoke('library_get_stats');
}

/** 首页洞察：4 个歌曲榜 + 艺人榜 + 专辑榜 + 今日次数 + 上次听歌，一次 IPC 打包 */
export function libraryGetInsights(): Promise<LibraryInsightsDTO> {
  return invoke('library_get_insights');
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

export function libraryRecordPlay(trackId: number, durationMs: number, mediaFileId?: number | null): Promise<void> {
  return invoke('library_record_play', { trackId, durationMs, mediaFileId });
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

export function libraryFetchMissingAlbumCover(albumId: number, allowOnline: boolean): Promise<number | null> {
  return invoke('library_fetch_missing_album_cover', { albumId, allowOnline });
}

export function libraryFetchMissingArtistCover(artistId: number, allowOnline: boolean): Promise<number | null> {
  return invoke('library_fetch_missing_artist_cover', { artistId, allowOnline });
}

export function libraryGetLyrics(trackId: number, allowOnline: boolean): Promise<string | null> {
  return invoke('library_get_lyrics', { trackId, allowOnline });
}

export function libraryGetTrackFileInfo(trackId: number): Promise<TrackFileInfoDTO | null> {
  return invoke('library_get_track_file_info', { trackId });
}


export function libraryGetCacheSize(): Promise<number> {
  return invoke('library_get_cache_size');
}

/** 智能歌单：按预设规则查询歌曲列表。kind 支持 most_played / recently_added / recently_played / never_played */
export function libraryGetSmartPlaylist(kind: string, limit?: number): Promise<TrackDTO[]> {
  return invoke('library_get_smart_playlist', { kind, limit });
}

/** 获取某首歌曲的所有可用物理文件版本（用于多音源版本切换 UI） */
export function libraryGetTrackVersions(trackId: number): Promise<TrackFileInfoDTO[]> {
  return invoke('library_get_track_versions', { trackId });
}

/** 设置某首歌曲的首选主文件版本 */
export function librarySetPrimaryFile(trackId: number, mediaFileId: number): Promise<void> {
  return invoke('library_set_primary_file', { trackId, mediaFileId });
}

export type PlayabilityState = 'local' | 'cached' | 'remote' | 'unavailable';

/** 曲库数据库文件总大小（字节，含 WAL/SHM） */
export function storageGetDbSize(): Promise<number> {
  return invoke('storage_get_db_size');
}

/** [MA3 A3-3] 批量查询歌曲的可播性状态 */
export function libraryGetPlayability(trackIds: number[]): Promise<Record<number, PlayabilityState>> {
  return invoke('library_get_playability', { trackIds });
}

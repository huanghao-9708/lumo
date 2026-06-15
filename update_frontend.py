import re

def update_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    content = re.sub(r"import \{ invoke \} from '\.\./utils/tauriInvoke';\n", '', content)
    content = re.sub(r"import \{ invoke \} from '@tauri-apps/api/core';\n", '', content)
    
    import_stmt = """
import {
  libraryGetTracks, libraryGetAlbums, libraryGetArtists, libraryGetAlbumTracks, libraryGetArtistAlbums, libraryGetArtistTracks, libraryGetArtistStats,
  libraryCreatePlaylist, libraryGetPlaylists, libraryAddToPlaylist, libraryGetPlaylistTracks, libraryDeletePlaylist, libraryRemovePlaylistItem, libraryAddFolderToPlaylist,
  libraryToggleFavorite, libraryRecordPlay, libraryGetRecentlyPlayed, libraryGetFavoriteTracks, librarySavePlayQueue, libraryGetPlayQueue,
  libraryClearCache, libraryGetFolderContents
} from '../api/library';
import {
  playbackPlay, playbackPause, playbackResume, playbackStop, playbackSetVolume, playbackGetPos, playbackSeek, playbackIsFinished
} from '../api/playback';
import {
  sourceAddLocal, sourceAddWebdav, sourceList, sourceRemove, sourceScan
} from '../api/scanner';
"""
    if 'player.ts' in filepath:
        content = content.replace("import { listen } from '@tauri-apps/api/event';", "import { listen } from '@tauri-apps/api/event';\n" + import_stmt)
    elif 'SettingsView.vue' in filepath:
        import_stmt_vue = import_stmt.replace("'../api", "'../../../../api")
        content = content.replace("import { invoke } from '@tauri-apps/api/core';", import_stmt_vue.strip())
        content = content.replace("import { invoke } from '../../../../utils/tauriInvoke';", import_stmt_vue.strip())

    replacements = [
        (r"invoke\('library_clear_cache'\)", "libraryClearCache()"),
        (r"invoke\('library_get_folder_contents',\s*\{\s*sourceId,\s*folderPath,\s*limit,\s*offset\s*\}\)", "libraryGetFolderContents(sourceId, folderPath, limit, offset)"),
        (r"invoke\('library_add_folder_to_playlist',\s*\{\s*sourceId,\s*folderPath,\s*playlistId\s*\}\)", "libraryAddFolderToPlaylist(sourceId, folderPath, playlistId)"),
        (r"invoke\('library_get_tracks',\s*\{\s*limit,\s*offset,\s*searchKeyword:\s*q\s*\}\)", "libraryGetTracks(limit, offset, q)"),
        (r"invoke\('library_get_playlists'\)", "libraryGetPlaylists()"),
        (r"invoke\('library_get_favorite_tracks'\)", "libraryGetFavoriteTracks()"),
        (r"invoke\('library_toggle_favorite',\s*\{\s*trackId,\s*isFavorite:\s*newStatus\s*\}\)", "libraryToggleFavorite(trackId, newStatus)"),
        (r"invoke\('library_record_play',\s*\{\s*trackId,\s*durationMs:\s*durationPlayed\s*\}\)", "libraryRecordPlay(trackId, durationPlayed)"),
        (r"invoke\('library_add_to_playlist',\s*\{\s*playlistId,\s*trackId\s*\}\)", "libraryAddToPlaylist(playlistId, trackId)"),
        (r"invoke\('library_get_playlist_tracks',\s*\{\s*playlistId\s*\}\)", "libraryGetPlaylistTracks(playlistId)"),
        (r"invoke\('library_get_recently_played',\s*\{\s*limit:\s*50\s*\}\)", "libraryGetRecentlyPlayed(50)"),
        (r"invoke\('library_get_albums',\s*\{\s*limit,\s*offset,\s*searchKeyword:\s*q\s*\}\)", "libraryGetAlbums(limit, offset, q)"),
        (r"invoke\('library_get_artists',\s*\{\s*limit,\s*offset,\s*searchKeyword:\s*q\s*\}\)", "libraryGetArtists(limit, offset, q)"),
        (r"invoke\('library_get_album_tracks',\s*\{\s*albumId:\s*newId\s*\}\)", "libraryGetAlbumTracks(newId)"),
        (r"invoke\('library_get_artist_tracks',\s*\{\s*artistId,\s*limit,\s*offset\s*\}\)", "libraryGetArtistTracks(artistId, limit, offset)"),
        (r"invoke\('library_get_artist_albums',\s*\{\s*artistId,\s*limit,\s*offset\s*\}\)", "libraryGetArtistAlbums(artistId, limit, offset)"),
        (r"invoke\('library_get_artist_stats',\s*\{\s*artistId:\s*newId\s*\}\)", "libraryGetArtistStats(newId)"),
        (r"invoke\('library_save_play_queue',\s*\{\s*trackIds:\s*queue\.value\.map\(t\s*=>\s*t\.id\)\s*\}\)", "librarySavePlayQueue(queue.value.map(t => t.id))"),
        (r"invoke\('library_get_play_queue'\)", "libraryGetPlayQueue()"),
        (r"invoke\('playback_set_volume',\s*\{\s*volume:\s*vol\s*/\s*100\s*\}\)", "playbackSetVolume(vol / 100)"),
        (r"invoke\('playback_set_volume',\s*\{\s*volume:\s*v\s*/\s*100\s*\}\)", "playbackSetVolume(v / 100)"),
        (r"invoke\('library_delete_playlist',\s*\{\s*playlistId\s*\}\)", "libraryDeletePlaylist(playlistId)"),
        (r"invoke\('library_remove_playlist_item',\s*\{\s*playlistId,\s*trackId\s*\}\)", "libraryRemovePlaylistItem(playlistId, trackId)"),
        (r"invoke\('playback_pause'\)", "playbackPause()"),
        (r"invoke\('playback_play',\s*\{\s*mediaFileId:\s*track\.primary_file_id\s*\}\)", "playbackPlay(track.primary_file_id)"),
        (r"invoke\('playback_seek',\s*\{\s*positionMs:\s*progressMs\.value\s*\}\)", "playbackSeek(progressMs.value)"),
        (r"invoke\('playback_seek',\s*\{\s*positionMs\s*\}\)", "playbackSeek(positionMs)"),
        (r"invoke\('playback_resume'\)", "playbackResume()"),
        (r"invoke\('source_add_local',\s*\{\s*path,\s*name\s*\}\)", "sourceAddLocal(path, name)"),
        (r"invoke\('source_add_webdav',\s*\{\s*url:\s*path,\s*name,\s*username,\s*password\s*\}\)", "sourceAddWebdav(path, name, username, password)"),
        (r"invoke\('source_list'\)", "sourceList()"),
        (r"invoke\('source_remove',\s*\{\s*sourceId:\s*id\s*\}\)", "sourceRemove(id)"),
        (r"invoke\('source_scan',\s*\{\s*sourceId:\s*id\s*\}\)", "sourceScan(id)"),
    ]

    for p, r in replacements:
        content = re.sub(p, r, content)

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

update_file('src/stores/player.ts')
update_file('src/plugins/ui-simple/views/SettingsView.vue')

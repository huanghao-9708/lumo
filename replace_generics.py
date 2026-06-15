import re

with open('src/stores/player.ts', 'r', encoding='utf-8') as f:
    content = f.read()

import_library = re.search(r"import \{([^}]+)\} from '\.\./api/library';", content)
if import_library and 'libraryGetLyrics' not in import_library.group(1):
    new_import = import_library.group(0).replace('}', ', libraryGetLyrics, libraryGetTrackFileInfo }')
    content = content.replace(import_library.group(0), new_import)

replacements = [
    (r"invoke<string \| null>\('library_get_lyrics',\s*\{\s*trackId:\s*newTrack\.id\s*\}\)", "libraryGetLyrics(newTrack.id)"),
    (r"invoke<any>\('library_get_track_file_info',\s*\{\s*trackId:\s*newTrack\.id\s*\}\)", "libraryGetTrackFileInfo(newTrack.id)"),
    (r"invoke<number>\('library_create_playlist',\s*\{\s*name,\s*description\s*\}\)", "libraryCreatePlaylist(name, description)"),
    (r"invoke<number>\('playback_get_pos'\)", "playbackGetPos()"),
    (r"invoke<boolean>\('playback_is_finished'\)", "playbackIsFinished()"),
    (r"invoke<number \| null>\('playback_play',\s*\{\s*mediaFileId:\s*track\.primary_file_id\s*\}\)", "playbackPlay(track.primary_file_id)"),
]

for p, r in replacements:
    content = re.sub(p, r, content)

with open('src/stores/player.ts', 'w', encoding='utf-8') as f:
    f.write(content)

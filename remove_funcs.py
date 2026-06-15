import re
import os

with open('src-tauri/src/services/library.rs', 'r', encoding='utf-8') as f:
    content = f.read()

def remove_func(name, text):
    pattern = r'(?:pub )?fn ' + name + r'\s*\(.*?\).*?\{'
    match = re.search(pattern, text, re.DOTALL)
    if not match: return text
    start = match.start()
    
    open_braces = 0
    in_str = False
    escape = False
    
    for i in range(start, len(text)):
        c = text[i]
        if escape:
            escape = False
            continue
        if c == '\\':
            escape = True
            continue
        if c == '"':
            in_str = not in_str
            continue
        
        if not in_str:
            if c == '{': open_braces += 1
            elif c == '}':
                open_braces -= 1
                if open_braces == 0:
                    return text[:start] + text[i+1:]
    return text

funcs = ['get_tracks_paginated', 'toggle_favorite', 'record_play', 'get_recently_played', 'get_favorite_tracks', 'get_track_by_path', 'save_play_queue', 'get_play_queue', 'get_folder_contents',
         'get_albums_paginated', 'get_album_tracks',
         'get_artists_paginated', 'get_artist_albums', 'get_artist_tracks', 'get_artist_stats',
         'create_playlist', 'get_playlists', 'add_to_playlist', 'get_playlist_tracks', 'delete_playlist', 'remove_playlist_item', 'add_folder_to_playlist']

for f in funcs:
    content = remove_func(f, content)

with open('src-tauri/src/services/library.rs', 'w', encoding='utf-8') as f:
    f.write(content)

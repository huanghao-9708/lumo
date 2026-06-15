import re
import os

with open('src-tauri/src/services/library.rs', 'r', encoding='utf-8') as f:
    content = f.read()

def extract_func(name):
    pattern = r'(?:pub )?fn ' + name + r'\s*\(.*?\).*?\{'
    match = re.search(pattern, content, re.DOTALL)
    if not match: return None
    start = match.start()
    
    open_braces = 0
    in_str = False
    escape = False
    
    for i in range(start, len(content)):
        c = content[i]
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
                    return content[start:i+1].strip()
    return None

track_funcs = ['get_tracks_paginated', 'toggle_favorite', 'record_play', 'get_recently_played', 'get_favorite_tracks', 'get_track_by_path', 'save_play_queue', 'get_play_queue', 'get_folder_contents']
album_funcs = ['get_albums_paginated', 'get_album_tracks']
artist_funcs = ['get_artists_paginated', 'get_artist_albums', 'get_artist_tracks', 'get_artist_stats']
playlist_funcs = ['create_playlist', 'get_playlists', 'add_to_playlist', 'get_playlist_tracks', 'delete_playlist', 'remove_playlist_item', 'add_folder_to_playlist']

def write_repo(name, funcs):
    out = 'use rusqlite::{Connection, params, OptionalExtension};\n'
    out += 'use crate::models::*;\n\n'
    out += f'pub struct {name};\n\n'
    out += f'impl {name} {{\n'
    for f in funcs:
        func_body = extract_func(f)
        if func_body:
            # We must make sure it is pub fn
            if not func_body.startswith('pub '):
                func_body = 'pub ' + func_body
            func_body = '\n'.join('    ' + line for line in func_body.split('\n'))
            out += func_body + '\n\n'
    out += '}\n'
    return out

with open('src-tauri/src/repositories/track_repo.rs', 'w', encoding='utf-8') as f: f.write(write_repo('TrackRepo', track_funcs))
with open('src-tauri/src/repositories/album_repo.rs', 'w', encoding='utf-8') as f: f.write(write_repo('AlbumRepo', album_funcs))
with open('src-tauri/src/repositories/artist_repo.rs', 'w', encoding='utf-8') as f: f.write(write_repo('ArtistRepo', artist_funcs))
with open('src-tauri/src/repositories/playlist_repo.rs', 'w', encoding='utf-8') as f: f.write(write_repo('PlaylistRepo', playlist_funcs))

with open('src-tauri/src/repositories/mod.rs', 'w', encoding='utf-8') as f:
    f.write('pub mod track_repo;\npub mod album_repo;\npub mod artist_repo;\npub mod playlist_repo;\n')

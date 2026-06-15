import re

with open('src/stores/player.ts', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace multi-line invokes
content = re.sub(r"invoke\('library_get_folder_contents',\s*\{\s*sourceId,\s*folderPath:\s*folderPath\s*\|\|\s*null,\s*limit,\s*offset\s*\}\)", "libraryGetFolderContents(sourceId, folderPath || undefined, limit, offset)", content)
content = re.sub(r"invoke\('library_get_tracks',\s*\{\s*limit:\s*tracksLimit,\s*offset:\s*tracksOffset,\s*searchKeyword:\s*tracksSearchKeyword\.value\s*\}\)", "libraryGetTracks(tracksLimit, tracksOffset, tracksSearchKeyword.value)", content)
content = re.sub(r"invoke\('library_get_albums',\s*\{\s*limit:\s*albumsLimit,\s*offset:\s*albumsOffset,\s*searchKeyword:\s*albumsSearchKeyword\.value\s*\}\)", "libraryGetAlbums(albumsLimit, albumsOffset, albumsSearchKeyword.value)", content)
content = re.sub(r"invoke\('library_get_artists',\s*\{\s*limit:\s*artistsLimit,\s*offset:\s*artistsOffset,\s*searchKeyword:\s*artistsSearchKeyword\.value\s*\}\)", "libraryGetArtists(artistsLimit, artistsOffset, artistsSearchKeyword.value)", content)

with open('src/stores/player.ts', 'w', encoding='utf-8') as f:
    f.write(content)

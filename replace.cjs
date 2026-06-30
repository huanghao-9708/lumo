const fs = require('fs');
const path = require('path');

const dir = path.join(__dirname, 'src/components/content');
const files = fs.readdirSync(dir).filter(f => f.endsWith('.vue'));

for (const file of files) {
  const filePath = path.join(dir, file);
  let content = fs.readFileSync(filePath, 'utf-8');

  // For `track` iterator
  content = content.replace(
    /<div class="flex-\[1(?:\.5)?\] min-w-0 hidden sm:block text-\[13px\] text-text-secondary truncate">{{ track\.artist }}<\/div>/g,
    `<div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate"><span class="hover:underline cursor-pointer" @click.stop="if(track.artistId) { playerStore.activeLibraryTab = '艺术家'; playerStore.activeArtistId = track.artistId; }">{{ track.artist }}</span></div>`
  );
  content = content.replace(
    /<div class="flex-\[1(?:\.5)?\] min-w-0 hidden md:block text-\[13px\] text-text-secondary truncate">{{ track\.album }}<\/div>/g,
    `<div class="flex-[1.5] min-w-0 hidden md:block text-[13px] text-text-secondary truncate"><span class="hover:underline cursor-pointer" @click.stop="if(track.albumId) { playerStore.activeLibraryTab = '专辑'; playerStore.activeAlbumId = track.albumId; }">{{ track.album }}</span></div>`
  );

  // For `song` iterator
  content = content.replace(
    /<div class="flex-\[1(?:\.5)?\] min-w-0 hidden sm:block text-\[13px\] text-text-secondary truncate">{{ song\.artist }}<\/div>/g,
    `<div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate"><span class="hover:underline cursor-pointer" @click.stop="if(song.artistId) { playerStore.activeLibraryTab = '艺术家'; playerStore.activeArtistId = song.artistId; }">{{ song.artist }}</span></div>`
  );
  content = content.replace(
    /<div class="flex-\[1(?:\.5)?\] min-w-0 hidden md:block text-\[13px\] text-text-secondary truncate">{{ song\.album }}<\/div>/g,
    `<div class="flex-[1.5] min-w-0 hidden md:block text-[13px] text-text-secondary truncate"><span class="hover:underline cursor-pointer" @click.stop="if(song.albumId) { playerStore.activeLibraryTab = '专辑'; playerStore.activeAlbumId = song.albumId; }">{{ song.album }}</span></div>`
  );

  fs.writeFileSync(filePath, content, 'utf-8');
}
console.log('Replaced track list fields.');

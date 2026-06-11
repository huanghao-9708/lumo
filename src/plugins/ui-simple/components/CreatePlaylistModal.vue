<script setup lang="ts">
import { ref } from 'vue';
import { usePlayerStore } from '../../../stores/player';

const playerStore = usePlayerStore();
const name = ref('');
const description = ref('');
const isSubmitting = ref(false);

const close = () => {
  playerStore.isCreatePlaylistModalOpen = false;
  name.value = '';
  description.value = '';
};

const submit = async () => {
  if (!name.value.trim() || isSubmitting.value) return;
  isSubmitting.value = true;
  try {
    const newId = await playerStore.createPlaylist(name.value.trim(), description.value.trim());
    close();
    playerStore.activePlaylistId = newId;
    playerStore.activeLibraryTab = '歌单详情';
  } catch(e) {
    console.error(e);
  } finally {
    isSubmitting.value = false;
  }
};
</script>

<template>
  <div v-if="playerStore.isCreatePlaylistModalOpen" class="fixed inset-0 z-50 flex items-center justify-center bg-[#fdfcf9]/80 backdrop-blur-sm" @click="close">
    <div class="bg-white border border-[#e8e6df] shadow-2xl p-10 w-full max-w-lg relative" @click.stop>
      <h2 class="font-serif italic text-4xl mb-8 tracking-tight text-black">New Playlist</h2>
      
      <div class="space-y-8">
        <div>
          <label class="block text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-2 uppercase">Name</label>
          <input 
            v-model="name"
            type="text" 
            placeholder="Name your playlist..."
            class="w-full bg-transparent border-b border-[#e8e6df] py-2 text-xl font-medium focus:outline-none focus:border-black transition-colors"
            @keyup.enter="submit"
            autofocus
          />
        </div>
        
        <div>
          <label class="block text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-2 uppercase">Description</label>
          <textarea 
            v-model="description"
            rows="3"
            placeholder="A brief description..."
            class="w-full bg-transparent border-b border-[#e8e6df] py-2 text-sm text-[#555] focus:outline-none focus:border-black transition-colors resize-none"
          ></textarea>
        </div>
      </div>

      <div class="mt-12 flex justify-end gap-8 items-center">
        <button @click="close" class="text-[12px] font-bold tracking-widest text-[#a0a0a0] hover:text-black uppercase transition-colors">
          Cancel
        </button>
        <button 
          @click="submit" 
          :disabled="!name.trim() || isSubmitting"
          class="text-[12px] font-bold tracking-widest uppercase transition-colors"
          :class="name.trim() && !isSubmitting ? 'text-black hover:opacity-70' : 'text-[#dcdad1] cursor-not-allowed'"
        >
          {{ isSubmitting ? 'Creating...' : 'Create' }}
        </button>
      </div>
    </div>
  </div>
</template>

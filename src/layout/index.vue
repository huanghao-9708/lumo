<script setup lang="ts">
import Sidebar from './Sidebar.vue';
import MainContent from './MainContent.vue';
import RightSidebar from './RightSidebar.vue';
import PlayerBar from './PlayerBar.vue';
import CreatePlaylistModal from './components/CreatePlaylistModal.vue';
import { usePlayerStore } from '../stores/player';

const playerStore = usePlayerStore();
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-bg-base   overflow-hidden text-text-main   font-sans antialiased selection:bg-[#333]/10 selection:text-accent">
    <!-- Main Workspace -->
    <div class="flex-1 flex overflow-hidden min-h-0">
      <Sidebar />
      <MainContent />
      <RightSidebar v-if="playerStore.isRightPanelOpen" />
    </div>
    
    <!-- Bottom Player -->
    <PlayerBar />

    <!-- Modals -->
    <CreatePlaylistModal />
  </div>
</template>

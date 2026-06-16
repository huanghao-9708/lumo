<script setup lang="ts">
import Sidebar from './Sidebar.vue';
import MainContent from './MainContent.vue';
import RightSidebar from './RightSidebar.vue';
import PlayerBar from './PlayerBar.vue';
import CreatePlaylistModal from './components/CreatePlaylistModal.vue';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-bg-base overflow-hidden text-text-main font-sans antialiased selection:bg-accent selection:text-white">
    <div class="flex-1 flex overflow-hidden">
      <!-- Sidebar -->
      <div class="w-64 shrink-0 flex flex-col bg-bg-base transition-colors relative z-10 border-r border-border-color/50">
        <Sidebar />
      </div>
      
      <!-- Main Content -->
      <div class="flex-1 flex flex-col bg-bg-base relative min-w-0 transition-colors z-0">
        <MainContent />
      </div>
      
      <!-- Right Sidebar -->
      <div v-if="playerStore.isRightPanelOpen" class="w-80 shrink-0 flex flex-col bg-bg-base transition-colors relative z-10 border-l border-border-color/50 shadow-[-10px_0_30px_rgba(0,0,0,0.02)]">
        <RightSidebar />
      </div>
    </div>
    
    <!-- Player Bar -->
    <PlayerBar />

    <!-- Modals -->
    <CreatePlaylistModal />
  </div>
</template>

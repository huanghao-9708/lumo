<script setup lang="ts">
import Sidebar from './Sidebar.vue';
import MainContent from './MainContent.vue';
import RightSidebar from './RightSidebar.vue';
import TePlayerBar from './PlayerBar.vue';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-bg-base overflow-hidden text-text-main font-mono antialiased selection:bg-accent selection:text-bg-base">
    
    <!-- Top Workspace (Sidebar + Main + RightSidebar) -->
    <!-- Add top border if we want a full border box, but usually screen edges don't need it -->
    <div class="flex-1 flex overflow-hidden border-b border-border-color">
      
      <!-- Left Sidebar -->
      <div class="w-64 shrink-0 flex flex-col border-r border-border-color bg-bg-base">
        <Sidebar />
      </div>
      
      <!-- Main Content -->
      <div class="flex-1 flex flex-col bg-bg-base relative min-w-0">
        <MainContent />
      </div>
      
      <!-- Right Sidebar -->
      <div v-if="playerStore.isRightPanelOpen" class="w-80 shrink-0 flex flex-col border-l border-border-color bg-bg-base">
        <RightSidebar />
      </div>
      
    </div>
    
    <!-- Bottom Player Bar -->
    <TePlayerBar />
    
  </div>
</template>

<style scoped>
/* Ensure the grid borders are sharp */
* {
  box-sizing: border-box;
}
</style>

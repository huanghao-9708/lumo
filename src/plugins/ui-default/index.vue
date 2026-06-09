<script setup lang="ts">
import Sidebar from './Sidebar.vue';
import MainContent from './MainContent.vue';
import RightSidebar from './RightSidebar.vue';
import PlayerBar from './PlayerBar.vue';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-white overflow-hidden text-[#333333] font-sans antialiased selection:bg-brand-orange/20 selection:text-brand-orange">
    <!-- Main Workspace -->
    <div class="flex-1 flex overflow-hidden">
      <Sidebar />
      <MainContent />
      <RightSidebar v-if="playerStore.isRightPanelOpen" />
    </div>
    
    <!-- Bottom Player -->
    <PlayerBar />
  </div>
</template>

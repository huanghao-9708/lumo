<script setup lang="ts">
import Sidebar from '../simple/Sidebar.vue'; // We can reuse Simple Sidebar or build an advanced one
import MainContent from '../simple/MainContent.vue';
import RightSidebar from '../simple/RightSidebar.vue';
import AdvancedPlayerBar from './PlayerBar.vue';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-gradient-to-br from-[#0f172a] to-[#1e293b] overflow-hidden text-white font-sans antialiased selection:bg-brand-orange/30 selection:text-white">
    <!-- Advanced Layout: Floating Workspace -->
    <div class="flex-1 flex overflow-hidden p-4 gap-4">
      <div class="w-64 rounded-2xl bg-white/5 backdrop-blur-xl border border-white/10 shadow-2xl overflow-hidden flex flex-col">
        <Sidebar />
      </div>
      
      <div class="flex-1 rounded-2xl bg-white/5 backdrop-blur-xl border border-white/10 shadow-2xl overflow-hidden relative">
        <MainContent />
      </div>
      
      <div v-if="playerStore.isRightPanelOpen" class="w-80 rounded-2xl bg-white/5 backdrop-blur-xl border border-white/10 shadow-2xl overflow-hidden">
        <RightSidebar />
      </div>
    </div>
    
    <!-- Advanced Bottom Player -->
    <AdvancedPlayerBar />
  </div>
</template>

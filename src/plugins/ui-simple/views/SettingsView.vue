<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { RefreshCw, Trash2, Power, PowerOff, Plus, X, ChevronDown, ChevronRight, HardDrive, Globe, Folder } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';
import { open } from '@tauri-apps/plugin-dialog';
import { libraryClearCache, libraryGetCacheSize } from '../../../api/library';
import { resetArtworkFrontCache } from '../../../composables/useArtworkSrc';

const playerStore = usePlayerStore();

// 折叠状态
const isLocalExpanded = ref(true);
const isWebdavExpanded = ref(true);

// 弹窗状态
const addModalType = ref<'none' | 'local' | 'webdav'>('none');
const newSourceName = ref('');
const newSourcePath = ref('');
const newSourceUsername = ref('');
const newSourcePassword = ref('');

// 图片缓存与偏好状态
const cacheSizeStr = ref('计算中...');
const audioQuality = ref('无损优先');
const fadeSeconds = ref(0);

const selectFolder = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
  });
  if (selected && typeof selected === 'string') {
    newSourcePath.value = selected;
  }
};

const openModal = (type: 'local' | 'webdav') => {
  newSourceName.value = '';
  newSourcePath.value = '';
  newSourceUsername.value = '';
  newSourcePassword.value = '';
  addModalType.value = type;
};

const closeModal = () => {
  addModalType.value = 'none';
};

const confirmAddSource = async () => {
  if (newSourceName.value.trim() && newSourcePath.value.trim()) {
    try {
      await playerStore.addSource(
        addModalType.value as 'local' | 'webdav', 
        newSourceName.value.trim(), 
        newSourcePath.value.trim(),
        newSourceUsername.value.trim(),
        newSourcePassword.value.trim()
      );
      closeModal();
    } catch (e) {
      alert("添加失败: " + e);
    }
  }
};

const fetchCacheSize = async () => {
  try {
    const bytes = await libraryGetCacheSize();
    if (bytes === 0) {
      cacheSizeStr.value = '0 MB';
    } else {
      cacheSizeStr.value = `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
    }
  } catch (e) {
    console.error("Failed to fetch cache size:", e);
    cacheSizeStr.value = '计算失败';
  }
};

const clearCache = async () => {
  if (confirm('确定要清空封面缓存吗？这将清理已缓存的封面图像并重新加载。')) {
    try {
      cacheSizeStr.value = '清理中...';
      await libraryClearCache();
      // 同步清掉前端内存里的 dataURL 缓存，否则已删除的后端封面还会被继续使用
      resetArtworkFrontCache();
      await fetchCacheSize();
      alert('清理缓存成功');
    } catch (e) {
      alert('清理缓存失败');
      await fetchCacheSize();
    }
  }
};

onMounted(() => {
  playerStore.fetchSources();
  fetchCacheSize();
  
  const savedQuality = localStorage.getItem('lumo_pref_audio_quality');
  if (savedQuality) {
    audioQuality.value = savedQuality;
  }
  
  const savedFade = localStorage.getItem('lumo_pref_fade_seconds');
  if (savedFade) {
    fadeSeconds.value = parseInt(savedFade, 10) || 0;
  }
});

watch(audioQuality, (newVal) => {
  localStorage.setItem('lumo_pref_audio_quality', newVal);
});

watch(fadeSeconds, (newVal) => {
  localStorage.setItem('lumo_pref_fade_seconds', String(newVal));
});
</script>

<template>
  <div class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-2 pt-2 pb-10">
    <div class="max-w-2xl">
      
      <!-- Section: Library & Sources -->
      <section class="mb-14">
        <h2 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] uppercase border-b border-[#e8e6df] mb-6 pb-2">曲库与来源</h2>
        
        <div class="space-y-8 flex flex-col">

          <!-- Local Folders Accordion -->
          <div>
            <div class="flex items-center justify-between mb-4">
              <button 
                @click="isLocalExpanded = !isLocalExpanded" 
                class="flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-black hover:text-[#555] transition-colors"
              >
                <ChevronDown v-if="isLocalExpanded" class="w-4 h-4 stroke-[1.5]" />
                <ChevronRight v-else class="w-4 h-4 stroke-[1.5]" />
                <HardDrive class="w-4 h-4 stroke-[1.5] mr-1" />
                本地音乐文件夹
              </button>
              <button 
                @click="openModal('local')"
                class="flex items-center gap-1 text-[10px] font-bold tracking-[0.2em] text-[#777] hover:text-black transition-colors uppercase group"
              >
                <Plus class="w-3.5 h-3.5 stroke-[1.5] group-hover:rotate-90 transition-transform" /> 添加
              </button>
            </div>
            
            <div v-show="isLocalExpanded" class="space-y-2">
              <div v-if="playerStore.localSources.length === 0" class="text-xs text-[#a0a0a0] py-4">未添加本地目录。</div>
              <div 
                v-for="source in playerStore.localSources" 
                :key="source.id"
                class="flex items-start justify-between group py-2"
                :class="!source.isEnabled ? 'opacity-40' : ''"
              >
                <div>
                  <div class="flex items-center gap-3 mb-1">
                    <h3 class="text-[13px] font-semibold text-black" :class="!source.isEnabled ? 'line-through' : ''">{{ source.name }}</h3>
                    <span v-if="!source.isEnabled" class="text-[9px] font-bold px-1.5 py-0.5 bg-[#dcdad1] text-white rounded-[2px] tracking-wider uppercase">已禁用</span>
                  </div>
                  <p class="text-xs text-[#777] font-mono tracking-tight mb-1">{{ source.path }}</p>
                  <p class="text-[10px] text-[#a0a0a0] uppercase tracking-widest">最后扫描: {{ source.lastScanned }}</p>
                </div>
                <!-- 操作按键 -->
                <div class="flex items-center gap-5 opacity-0 group-hover:opacity-100 transition-opacity mt-1">
                  <button @click="playerStore.scanSource(source.id)" class="text-[#777] hover:text-black transition-colors" :disabled="!source.isEnabled" title="Rescan"><RefreshCw class="w-[14px] h-[14px] stroke-[1.5]" /></button>
                  <button @click="playerStore.toggleSource(source.id)" class="text-[#777] hover:text-black transition-colors" :title="source.isEnabled ? 'Disable' : 'Enable'">
                    <PowerOff v-if="source.isEnabled" class="w-[14px] h-[14px] stroke-[1.5]" />
                    <Power v-else class="w-[14px] h-[14px] stroke-[1.5]" />
                  </button>
                  <button @click="playerStore.removeSource(source.id)" class="text-[#777] hover:text-red-500 transition-colors" title="Remove"><Trash2 class="w-[14px] h-[14px] stroke-[1.5]" /></button>
                </div>
              </div>
            </div>
          </div>

          <!-- WebDAV Sources Accordion -->
          <div>
            <div class="flex items-center justify-between mb-4">
              <button 
                @click="isWebdavExpanded = !isWebdavExpanded" 
                class="flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-black hover:text-[#555] transition-colors"
              >
                <ChevronDown v-if="isWebdavExpanded" class="w-4 h-4 stroke-[1.5]" />
                <ChevronRight v-else class="w-4 h-4 stroke-[1.5]" />
                <Globe class="w-4 h-4 stroke-[1.5] mr-1" />
                WebDAV 远程来源
              </button>
              <button 
                @click="openModal('webdav')"
                class="flex items-center gap-1 text-[10px] font-bold tracking-[0.2em] text-[#777] hover:text-black transition-colors uppercase group"
              >
                <Plus class="w-3.5 h-3.5 stroke-[1.5] group-hover:rotate-90 transition-transform" /> 添加
              </button>
            </div>
            
            <div v-show="isWebdavExpanded" class="space-y-2">
              <div v-if="playerStore.webdavSources.length === 0" class="text-xs text-[#a0a0a0] py-4">未添加远程来源。</div>
              <div 
                v-for="source in playerStore.webdavSources" 
                :key="source.id"
                class="flex items-start justify-between group py-2"
                :class="!source.isEnabled ? 'opacity-40' : ''"
              >
                <div>
                  <div class="flex items-center gap-3 mb-1">
                    <h3 class="text-[13px] font-semibold text-black" :class="!source.isEnabled ? 'line-through' : ''">{{ source.name }}</h3>
                    <span v-if="!source.isEnabled" class="text-[9px] font-bold px-1.5 py-0.5 bg-[#dcdad1] text-white rounded-[2px] tracking-wider uppercase">已禁用</span>
                  </div>
                  <p class="text-xs text-[#777] font-mono tracking-tight mb-1">{{ source.path }} <span v-if="source.username" class="ml-2 text-[#a0a0a0]">({{ source.username }})</span></p>
                  <p class="text-[10px] text-[#a0a0a0] uppercase tracking-widest">最后扫描: {{ source.lastScanned }}</p>
                </div>
                <!-- 操作按键 -->
                <div class="flex items-center gap-5 opacity-0 group-hover:opacity-100 transition-opacity mt-1">
                  <button @click="playerStore.scanSource(source.id)" class="text-[#777] hover:text-black transition-colors" :disabled="!source.isEnabled" title="Rescan"><RefreshCw class="w-[14px] h-[14px] stroke-[1.5]" /></button>
                  <button @click="playerStore.toggleSource(source.id)" class="text-[#777] hover:text-black transition-colors" :title="source.isEnabled ? 'Disable' : 'Enable'">
                    <PowerOff v-if="source.isEnabled" class="w-[14px] h-[14px] stroke-[1.5]" />
                    <Power v-else class="w-[14px] h-[14px] stroke-[1.5]" />
                  </button>
                  <button @click="playerStore.removeSource(source.id)" class="text-[#777] hover:text-red-500 transition-colors" title="Remove"><Trash2 class="w-[14px] h-[14px] stroke-[1.5]" /></button>
                </div>
              </div>
            </div>
          </div>

        </div>
      </section>

      <!-- Section: Playback -->
      <section class="mb-14">
        <h2 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-6 uppercase border-b border-[#e8e6df] pb-2">播放与音频</h2>
        
        <div class="space-y-6">
          <div class="flex items-center justify-between group">
            <h3 class="text-[13px] font-semibold text-black">音质偏好</h3>
            <select v-model="audioQuality" class="bg-transparent text-xs text-[#777] font-medium uppercase tracking-widest focus:outline-none cursor-pointer text-right">
              <option value="无损优先">无损优先</option>
              <option value="高品质 (320kbps)">高品质 (320kbps)</option>
              <option value="标准">标准</option>
            </select>
          </div>

          <div class="flex items-center justify-between group">
            <div>
              <h3 class="text-[13px] font-semibold text-black mb-1">淡入淡出</h3>
              <p class="text-xs text-[#777]">歌曲之间平滑过渡</p>
            </div>
            <div class="flex items-center gap-2">
              <input v-model.number="fadeSeconds" type="range" class="w-24 accent-black" min="0" max="12" />
              <span class="text-xs font-bold text-black w-6 text-right">{{ fadeSeconds }}s</span>
            </div>
          </div>
        </div>
      </section>

      <!-- Section: Storage -->
      <section>
        <h2 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-6 uppercase border-b border-[#e8e6df] pb-2">存储与缓存</h2>
        
        <div class="flex items-start justify-between group">
          <div>
            <h3 class="text-[13px] font-semibold text-black mb-1">封面缓存</h3>
            <p class="text-xs text-[#777]">已使用 {{ cacheSizeStr }} 磁盘空间</p>
          </div>
          <button @click="clearCache" class="flex items-center gap-2 text-xs font-medium text-red-400 hover:text-red-600 transition-colors">
            <Trash2 class="w-4 h-4 stroke-[1.5]" />
            <span>清理</span>
          </button>
        </div>
      </section>
      
    </div>
  </div>

  <!-- Global Modal for Adding Source -->
  <Teleport to="body">
    <div v-if="addModalType !== 'none'" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div class="absolute inset-0 bg-white/60 backdrop-blur-sm" @click="closeModal"></div>
      
      <div class="relative w-full max-w-md bg-[#f9f8f6] shadow-xl border border-[#e8e6df] p-8 flex flex-col gap-6">
        <div class="flex items-center justify-between">
          <h2 class="font-serif italic text-2xl text-black">新增 {{ addModalType === 'local' ? '本地目录' : '远程来源' }}</h2>
          <button @click="closeModal" class="text-[#888] hover:text-black transition-colors"><X class="w-5 h-5 stroke-[1.5]" /></button>
        </div>

        <div class="space-y-5">
          <div>
            <label class="block text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-2 uppercase">别名 (Name)</label>
            <input v-model="newSourceName" type="text" :placeholder="addModalType === 'local' ? '例如: 高解析度音乐' : '例如: 群晖 NAS'" class="w-full bg-transparent border-b border-[#dcdad1] focus:border-black text-sm pb-1 focus:outline-none transition-colors" />
          </div>
          
          <div>
            <label class="block text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-2 uppercase">{{ addModalType === 'local' ? '系统路径' : '服务器地址' }}</label>
            <div class="relative flex items-center">
              <input 
                v-model="newSourcePath" 
                type="text" 
                :placeholder="addModalType === 'local' ? '点击图标选择目录...' : 'https://...'" 
                :readonly="addModalType === 'local'"
                class="w-full bg-transparent border-b border-[#dcdad1] focus:border-black text-sm pb-1 pr-6 focus:outline-none transition-colors"
                :class="addModalType === 'local' ? 'cursor-pointer' : ''"
                @click="addModalType === 'local' ? selectFolder() : null"
              />
              <button 
                v-if="addModalType === 'local'" 
                @click="selectFolder"
                class="absolute right-0 bottom-1.5 text-[#777] hover:text-black transition-colors"
              >
                <Folder class="w-4 h-4 stroke-[1.5]" />
              </button>
            </div>
          </div>

          <template v-if="addModalType === 'webdav'">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-2 uppercase">用户名</label>
                <input v-model="newSourceUsername" type="text" class="w-full bg-transparent border-b border-[#dcdad1] focus:border-black text-sm pb-1 focus:outline-none transition-colors" />
              </div>
              <div>
                <label class="block text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-2 uppercase">密码</label>
                <input v-model="newSourcePassword" type="password" class="w-full bg-transparent border-b border-[#dcdad1] focus:border-black text-sm pb-1 focus:outline-none transition-colors" />
              </div>
            </div>
          </template>
        </div>

        <div class="mt-4 flex justify-end">
          <button @click="confirmAddSource" class="bg-black text-white px-6 py-2 text-xs font-bold tracking-widest uppercase hover:bg-[#333] transition-colors">
            保存
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

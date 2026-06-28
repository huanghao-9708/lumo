import { defineStore } from "pinia";
import { ref } from "vue";

export const useUiStore = defineStore("ui", () => {
  // 右侧边栏显示状态
  const isRightSidebarVisible = ref(true);

  function toggleRightSidebar() {
    isRightSidebarVisible.value = !isRightSidebarVisible.value;
  }
  
  function setRightSidebarVisible(visible: boolean) {
    isRightSidebarVisible.value = visible;
  }

  return {
    isRightSidebarVisible,
    toggleRightSidebar,
    setRightSidebarVisible
  };
});

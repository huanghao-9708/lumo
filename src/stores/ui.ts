import { defineStore } from "pinia";
import { ref } from "vue";

export const useUiStore = defineStore("ui", () => {
  // 当前激活的 UI 插件，默认为复古杂志面板 'ui-simple'
  const activePlugin = ref("ui-simple");

  function setActivePlugin(name: string) {
    activePlugin.value = name;
  }

  return {
    activePlugin,
    setActivePlugin,
  };
});

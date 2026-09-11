/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

/** vite.config.ts 读 package.json 的 version 经 define 注入（发版只改 package.json） */
declare const __APP_VERSION__: string;

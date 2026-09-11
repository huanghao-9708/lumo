import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import pkg from "./package.json";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), tailwindcss()],

  // 版本号单一来源：读 package.json 的 version 注入全局常量 __APP_VERSION__
  // （类型声明在 src/vite-env.d.ts，消费方在 src/config/appInfo.ts）。
  // 以后发版只需改 package.json，UI 自动跟随，不会再漂移。
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));

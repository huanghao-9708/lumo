import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import pkg from "./package.json";

/**
 * 前端单元测试配置（I1/QA-005）。
 *
 * 与 vite.config.ts 分开的两个原因：
 * 1. 测试不需要 tailwind 与 dev server 配置，减少无关副作用；
 * 2. 需要 happy-dom 环境来挂载组件。
 *
 * __APP_VERSION__ 在 vite.config.ts 里由 define 注入，被测代码若间接引用
 * src/config/appInfo.ts 会拿到未定义的全局量，所以这里同样注入。
 */
export default defineConfig({
  plugins: [vue()],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  test: {
    environment: "happy-dom",
    // scripts/** 也纳入：联网门禁的判定逻辑是纯函数（CR-008），
    // 没有正反例锁住，它就只是一段没人敢改的字符串匹配。
    include: ["src/**/*.spec.ts", "scripts/**/*.spec.mjs"],
  },
});

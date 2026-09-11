/**
 * 应用元信息唯一出口。
 *
 * UI 里禁止硬编码版本号/介绍文案——统一从这里引用；
 * 版本号来自 vite.config.ts 读 package.json 注入的 __APP_VERSION__，
 * 以后发版只需改 package.json，桌面端与移动端 UI 自动跟随。
 */
export const APP_NAME = 'LUMO';
export const APP_NAME_CN = 'LUMO 轻音';
export const APP_TAGLINE = '本地音乐播放器 · 你的音乐，只属于你';
export const APP_DESCRIPTION = '一个轻量、温暖的本地音乐播放器';
export const APP_VERSION = __APP_VERSION__;
export const APP_VERSION_LABEL = `v${APP_VERSION}`;

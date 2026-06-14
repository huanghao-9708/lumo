/**
 * 根据当前操作系统生成 artwork 的可访问 URL。
 *
 * Tauri v2 的自定义 URI scheme 在不同平台的 WebView 里被规范化成不同的形式：
 *   - Windows (WebView2)：`http://<scheme>.localhost/<path>`
 *   - macOS (WKWebView) / Linux (WebKitGTK)：`<scheme>://<path>`
 *
 * 判断依据：Tauri 在 WebView 的 `navigator.userAgent` 中会注入平台信息
 * （"Windows"、"Macintosh" 等是 WebView 本身的标准片段，稳定性较高）。
 * 我们在首次调用时把结果缓存起来，避免每次都重新解析 UA 字符串。
 *
 * 注：不引入 `@tauri-apps/plugin-os` 以遵守项目"非必要不加依赖"的约束。
 */

export type Platform = 'windows' | 'macos' | 'linux' | 'unknown';

let cachedPlatform: Platform | null = null;

function detectPlatform(): Platform {
  if (cachedPlatform !== null) return cachedPlatform;
  const ua = (typeof navigator !== 'undefined' && navigator.userAgent) || '';
  // 注意大小写：WebView2 的 UA 里包含字面量 "Windows"，WKWebView 包含 "Macintosh"
  const platform: Platform = /Windows/.test(ua) ? 'windows'
    : /Macintosh|Mac OS X/.test(ua) ? 'macos'
    : /Linux/.test(ua) ? 'linux'
    : 'unknown';
  cachedPlatform = platform;
  return platform;
}

/** 暴露当前平台判定，便于其他模块复用（例如决定路径分隔符） */
export function getCurrentPlatform(): Platform {
  return detectPlatform();
}

export function getArtworkUrl(artworkId: number | string): string {
  // Windows 上的 WebView2 严格要求 http://lumo.localhost/ 形式
  if (detectPlatform() === 'windows') {
    return `http://lumo.localhost/artwork/${artworkId}`;
  }
  return `lumo://artwork/${artworkId}`;
}

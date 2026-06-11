export function getArtworkUrl(artworkId: number | string): string {
  // Tauri v2 on Windows uses http://<scheme>.localhost/
  // On macOS/Linux, it uses <scheme>://
  // We can detect Windows via navigator.userAgent or Tauri's API,
  // but since standard fetch works with http://lumo.localhost on Windows
  // and we are cross-platform, we can rely on Tauri's platform detection.
  // Actually, Tauri v2's WebView2 strictly requires http://lumo.localhost.
  const isWindows = navigator.userAgent.includes('Windows');
  if (isWindows) {
    return `http://lumo.localhost/artwork/${artworkId}`;
  }
  return `lumo://artwork/${artworkId}`;
}

export type Platform = "windows" | "macos" | "linux";

// The webview's user agent names the system: WebView2 on Windows, WKWebView
// on macOS, WebKitGTK on Linux.
function detectPlatform(): Platform {
  const agent = navigator.userAgent;
  if (agent.includes("Windows")) {
    return "windows";
  }
  if (agent.includes("Macintosh")) {
    return "macos";
  }
  return "linux";
}

export const platform: Platform = detectPlatform();

/**
 * Whether the app draws its own title bar. Only on Windows: there the window
 * is undecorated so the WinUI-style bar can sit on the Mica backdrop. macOS
 * and Linux keep the system's own title bar.
 */
export const drawsOwnTitleBar = platform === "windows";

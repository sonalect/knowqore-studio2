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
 * Whether the app draws its own title bar, so it can hold the app's own
 * buttons. On Windows and Linux the window is undecorated and the bar follows
 * the system's look; macOS keeps its own title bar.
 */
export const drawsOwnTitleBar = platform !== "macos";

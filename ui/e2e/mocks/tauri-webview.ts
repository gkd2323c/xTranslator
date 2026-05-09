/**
 * Mock for @tauri-apps/api/webview
 */

export interface Webview {
  label: string;
}

export async function getCurrentWebview(): Promise<Webview> {
  return { label: "mock" };
}

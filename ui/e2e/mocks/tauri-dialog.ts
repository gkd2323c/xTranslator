/**
 * Mock for @tauri-apps/plugin-dialog
 * Returns sensible defaults for E2E tests.
 */

import type { OpenDialogOptions, SaveDialogOptions } from "@tauri-apps/plugin-dialog";

/**
 * Open a file/directory selection dialog.
 * In mock mode, return a default path so panel operations can proceed.
 */
export async function open(options?: OpenDialogOptions): Promise<string | string[] | null> {
  // Return a single default path — the invoke mock will return data for this path
  const defaultPath = "C:/mock/input.txt";

  if (options?.multiple) {
    return [defaultPath];
  }

  if (options?.directory) {
    return "C:/mock/Voice";
  }

  return defaultPath;
}

/**
 * Save dialog — returns a default path.
 */
export async function save(options?: SaveDialogOptions): Promise<string | null> {
  return "C:/mock/output.txt";
}

/**
 * Ask dialog — returns true by default.
 */
export async function ask(message: string): Promise<boolean> {
  return true;
}

/**
 * Message dialog — no-op.
 */
export async function message(message: string): Promise<void> {
  // no-op
}

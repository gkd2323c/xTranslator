/**
 * Mock for @tauri-apps/api/event
 */

type UnlistenFn = () => void;
type EventCallback<T> = (event: { payload: T }) => void;

/** Listen to a Tauri event. In mock mode, returns a no-op unlisten function. */
export async function listen<T>(
  _event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  return () => {
    // no-op cleanup
  };
}

export type { UnlistenFn };

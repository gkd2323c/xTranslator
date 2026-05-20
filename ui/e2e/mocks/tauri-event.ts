/**
 * Mock for @tauri-apps/api/event
 * Supports test-driven event dispatching for E2E tests.
 */

type UnlistenFn = () => void;
type EventCallback<T> = (event: { payload: T }) => void;

// Registry of active listeners per event name — allows tests to dispatch events
const listeners = new Map<string, Set<EventCallback<any>>>();

/** Listen to a Tauri event. Returns cleanup function. */
export async function listen<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  if (!listeners.has(event)) {
    listeners.set(event, new Set());
  }
  listeners.get(event)!.add(handler);

  return () => {
    listeners.get(event)?.delete(handler);
  };
}

/**
 * Dispatch a synthetic Tauri event (for test use).
 * Simulates the backend emitting an event that the frontend listens to.
 */
export function __dispatch<T>(event: string, payload: T): void {
  const handlers = listeners.get(event);
  if (handlers) {
    handlers.forEach((handler) => handler({ payload }));
  }
}

/** Clear all registered listeners (call between tests). */
export function __resetListeners(): void {
  listeners.clear();
}

// Expose dispatch globally so Playwright tests can call via page.evaluate()
if (typeof window !== "undefined") {
  (window as any).__tauriEventDispatch = __dispatch;
  (window as any).__tauriEventReset = __resetListeners;
}

export type { UnlistenFn };

import type { GameSelectionMode, SupportedGameId } from "./api/strings";

export interface StoredGameSelection {
  gameSelectionMode: GameSelectionMode;
  currentGame: SupportedGameId | null;
}

/** Returns the explicit game for this load; Auto mode delegates to TES4 detection. */
export function requestedGameForLoad(
  mode: GameSelectionMode,
  currentGame: SupportedGameId | null,
): SupportedGameId | undefined {
  return mode === "manual" ? currentGame ?? undefined : undefined;
}

/** Restores selection from config; manual without `last_game` safely falls back to auto. */
export function restoreGameSelection(
  mode: GameSelectionMode | undefined,
  lastGame: SupportedGameId | undefined,
): StoredGameSelection {
  if (mode === "manual" && lastGame) {
    return { gameSelectionMode: "manual", currentGame: lastGame };
  }
  return { gameSelectionMode: "auto", currentGame: null };
}

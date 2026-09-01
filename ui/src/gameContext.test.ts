import { describe, expect, it } from "vitest";
import { requestedGameForLoad, restoreGameSelection } from "./gameContext";

describe("requestedGameForLoad", () => {
  it("leaves game unset in auto mode so the backend can inspect TES4", () => {
    expect(requestedGameForLoad("auto", "Fallout4")).toBeUndefined();
  });

  it("passes the explicit workspace in manual mode", () => {
    expect(requestedGameForLoad("manual", "Starfield")).toBe("Starfield");
  });

  it("does not invent SkyrimSE when manual mode has no game", () => {
    expect(requestedGameForLoad("manual", null)).toBeUndefined();
  });
});

describe("restoreGameSelection", () => {
  it("restores an explicit workspace", () => {
    expect(restoreGameSelection("manual", "Fallout76")).toEqual({
      gameSelectionMode: "manual",
      currentGame: "Fallout76",
    });
  });

  it("keeps auto mode even when a previous last_game exists", () => {
    expect(restoreGameSelection("auto", "SkyrimSE")).toEqual({
      gameSelectionMode: "auto",
      currentGame: null,
    });
  });

  it("falls back to auto instead of inventing a manual workspace", () => {
    expect(restoreGameSelection("manual", undefined)).toEqual({
      gameSelectionMode: "auto",
      currentGame: null,
    });
  });
});

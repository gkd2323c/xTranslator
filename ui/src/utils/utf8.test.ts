import { describe, expect, it } from "vitest";
import {
  replaceUtf8ByteRange,
  utf8ByteOffsetToCodeUnitIndex,
} from "./utf8";

describe("utf8ByteOffsetToCodeUnitIndex", () => {
  it("maps ASCII byte offsets directly", () => {
    expect(utf8ByteOffsetToCodeUnitIndex("test", 0)).toBe(0);
    expect(utf8ByteOffsetToCodeUnitIndex("test", 2)).toBe(2);
    expect(utf8ByteOffsetToCodeUnitIndex("test", 4)).toBe(4);
  });

  it("maps multibyte CJK boundaries to code unit indices", () => {
    expect(utf8ByteOffsetToCodeUnitIndex("A中B", 1)).toBe(1);
    expect(utf8ByteOffsetToCodeUnitIndex("A中B", 4)).toBe(2);
    expect(utf8ByteOffsetToCodeUnitIndex("A中B", 5)).toBe(3);
  });

  it("maps surrogate-pair boundaries correctly", () => {
    expect(utf8ByteOffsetToCodeUnitIndex("🙂a", 4)).toBe(2);
    expect(utf8ByteOffsetToCodeUnitIndex("🙂a", 5)).toBe(3);
  });
});

describe("replaceUtf8ByteRange", () => {
  it("replaces ranges after a multibyte prefix", () => {
    expect(replaceUtf8ByteRange("中test文", 3, 7, "best")).toBe("中best文");
  });

  it("replaces emoji-adjacent ranges without corrupting the string", () => {
    expect(replaceUtf8ByteRange("🙂test!", 4, 8, "best")).toBe("🙂best!");
  });
});

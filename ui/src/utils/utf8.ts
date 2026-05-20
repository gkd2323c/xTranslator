const utf8Encoder = new TextEncoder();

export function utf8ByteOffsetToCodeUnitIndex(
  text: string,
  byteOffset: number,
): number {
  const clampedOffset = Math.max(0, byteOffset);
  let consumedBytes = 0;
  let codeUnitIndex = 0;

  for (const char of text) {
    if (consumedBytes >= clampedOffset) {
      break;
    }

    consumedBytes += utf8Encoder.encode(char).length;
    codeUnitIndex += char.length;

    if (consumedBytes >= clampedOffset) {
      break;
    }
  }

  return codeUnitIndex;
}

export function utf8ByteRangeToCodeUnitRange(
  text: string,
  startByte: number,
  endByte: number,
): [number, number] {
  const safeStart = Math.max(0, startByte);
  const safeEnd = Math.max(safeStart, endByte);

  return [
    utf8ByteOffsetToCodeUnitIndex(text, safeStart),
    utf8ByteOffsetToCodeUnitIndex(text, safeEnd),
  ];
}

export function replaceUtf8ByteRange(
  text: string,
  startByte: number,
  endByte: number,
  replacement: string,
): string {
  const [startIndex, endIndex] = utf8ByteRangeToCodeUnitRange(
    text,
    startByte,
    endByte,
  );

  return text.slice(0, startIndex) + replacement + text.slice(endIndex);
}

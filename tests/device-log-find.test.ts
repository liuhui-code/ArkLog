import { describe, expect, it } from "vitest";
import { DeviceLogFind } from "../src/device-log-find";

describe("DeviceLogFind", () => {
  it("finds case-insensitive literal occurrences in display order", () => {
    const find = new DeviceLogFind("a.b", ["A.B then a.b", "axb", "a.b"]);

    expect(find.occurrences).toEqual([
      { lineIndex: 0, start: 0, end: 3, ordinal: 0 },
      { lineIndex: 0, start: 9, end: 12, ordinal: 1 },
      { lineIndex: 2, start: 0, end: 3, ordinal: 2 },
    ]);
    expect(find.occurrencesForLine(0)).toHaveLength(2);
  });

  it("wraps forward and backward navigation", () => {
    const find = new DeviceLogFind("match", ["match", "another match"]);

    expect(find.move(0, 1)).toBe(1);
    expect(find.move(1, 1)).toBe(0);
    expect(find.move(0, -1)).toBe(1);
  });

  it("has a stable empty state", () => {
    const find = new DeviceLogFind("missing", ["visible line"]);

    expect(find.count).toBe(0);
    expect(find.normalize(12)).toBe(0);
    expect(find.move(0, 1)).toBe(0);
  });
});

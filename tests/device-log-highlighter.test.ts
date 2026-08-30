import { describe, expect, it } from "vitest";
import { DeviceLogHighlighter } from "../src/device-log-highlighter";

describe("DeviceLogHighlighter", () => {
  it("combines overlapping decorations into lossless non-overlapping segments", () => {
    const raw = "error: disk failed";
    const highlighter = new DeviceLogHighlighter(raw, [
      { start: 0, end: 11, className: "regex-match" },
      { start: 7, end: 11, className: "find-match is-current" },
    ]);

    const segments = highlighter.segments();

    expect(segments).toEqual([
      { start: 0, end: 7, className: "regex-match" },
      { start: 7, end: 11, className: "regex-match find-match is-current" },
      { start: 11, end: 18, className: "" },
    ]);
    expect(segments.map(({ start, end }) => raw.slice(start, end)).join("")).toBe(raw);
  });

  it("ignores empty and out-of-bounds decorations", () => {
    const highlighter = new DeviceLogHighlighter("raw", [
      { start: 0, end: 0, className: "empty" },
      { start: -1, end: 2, className: "outside" },
      { start: 1, end: 4, className: "outside" },
    ]);

    expect(highlighter.segments()).toEqual([{ start: 0, end: 3, className: "" }]);
  });
});

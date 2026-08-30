import { describe, expect, it } from "vitest";
import { DeviceLogEntry } from "../src/device-log-entry";
import { DeviceLogFilter, EMPTY_DEVICE_LOG_FILTER } from "../src/device-log-filter";

const entry = (raw: string) => DeviceLogEntry.parse(raw);

describe("DeviceLogFilter", () => {
  it("always interprets the query as a regular expression", () => {
    const filter = new DeviceLogFilter({
      ...EMPTY_DEVICE_LOG_FILTER,
      query: String.raw`task-\d+$`,
    });

    expect(filter.matches(entry("task-12"))).toBe(true);
    expect(filter.matches(entry("task pending"))).toBe(false);
  });

  it("uses the regular expression's case-sensitive semantics", () => {
    const filter = new DeviceLogFilter({
      ...EMPTY_DEVICE_LOG_FILTER,
      query: "error",
    });

    expect(filter.matches(entry("ERROR loading page"))).toBe(false);
    expect(filter.matches(entry("error loading page"))).toBe(true);
  });

  it("preserves whitespace in the entered regular expression", () => {
    const filter = new DeviceLogFilter({ query: " raw$" });

    expect(filter.matches(entry("raw"))).toBe(false);
    expect(filter.matches(entry("device raw"))).toBe(true);
  });

  it("matches a complete raw line longer than 4,096 characters", () => {
    const filter = new DeviceLogFilter({
      ...EMPTY_DEVICE_LOG_FILTER,
      query: "target$",
    });
    const longLine = `${"a".repeat(4_097)} target`;

    expect(filter.matches(entry(longLine))).toBe(true);
    expect(entry(longLine).raw).toBe(longLine);
  });

  it("returns every non-empty full-match range", () => {
    const filter = new DeviceLogFilter({ query: String.raw`task-\d+` });
    const log = entry("task-12 before task-345");

    expect(filter.matchRanges(log)).toEqual([
      { start: 0, end: 7 },
      { start: 15, end: 23 },
    ]);
  });

  it("keeps zero-width matches visible without empty highlight ranges", () => {
    const filter = new DeviceLogFilter({ query: "^" });
    const log = entry("raw line");

    expect(filter.matches(log)).toBe(true);
    expect(filter.matchRanges(log)).toEqual([]);
  });

  it("reports an invalid expression without matching raw lines", () => {
    const filter = new DeviceLogFilter({
      ...EMPTY_DEVICE_LOG_FILTER,
      query: "(",
    });

    expect(filter.error).toMatch(/Invalid regular expression/u);
    expect(filter.matches(entry("raw line"))).toBe(false);
    expect(filter.matchRanges(entry("raw line"))).toEqual([]);
  });
});

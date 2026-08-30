import { describe, expect, it } from "vitest";
import { DeviceLogVirtualWindow } from "../src/device-log-virtual-window";

describe("DeviceLogVirtualWindow", () => {
  const window = new DeviceLogVirtualWindow(20, 2, 10);

  it("returns an empty bounded range for an empty stream", () => {
    expect(window.range({
      totalRows: 0,
      scrollTop: 0,
      viewportHeight: 100,
      followingLatest: true,
    })).toEqual({ start: 0, end: 0, rowHeight: 20, totalHeight: 0 });
  });

  it("keeps the latest rows mounted without exceeding the DOM cap", () => {
    expect(window.range({
      totalRows: 1_000,
      scrollTop: 0,
      viewportHeight: 400,
      followingLatest: true,
    })).toEqual({ start: 990, end: 1_000, rowHeight: 20, totalHeight: 20_000 });
  });

  it("overscans around a paused scroll position", () => {
    expect(window.range({
      totalRows: 1_000,
      scrollTop: 4_000,
      viewportHeight: 100,
      followingLatest: false,
    })).toEqual({ start: 198, end: 207, rowHeight: 20, totalHeight: 20_000 });
  });

  it("centers an offscreen find occurrence and clamps either edge", () => {
    expect(window.range({
      totalRows: 1_000,
      scrollTop: 0,
      viewportHeight: 100,
      followingLatest: false,
      anchorIndex: 500,
    })).toMatchObject({ start: 496, end: 505 });
    expect(window.range({
      totalRows: 1_000,
      scrollTop: 0,
      viewportHeight: 100,
      followingLatest: false,
      anchorIndex: 999,
    })).toMatchObject({ start: 991, end: 1_000 });
  });
});

import { describe, expect, it } from "vitest";
import { DeviceLogEntry } from "../src/device-log-entry";

describe("DeviceLogEntry", () => {
  it("parses every structured field from a standard HiLog line", () => {
    const entry = DeviceLogEntry.parse(
      "08-29 20:00:01.234 1201 1202 I C03F00/AppTag com.example.demo: page ready\n",
    );

    expect(entry).toMatchObject({
      raw: "08-29 20:00:01.234 1201 1202 I C03F00/AppTag com.example.demo: page ready",
      timestamp: "08-29 20:00:01.234",
      pid: 1201,
      tid: 1202,
      level: "info",
      domain: "C03F00",
      tag: "AppTag",
      process: "com.example.demo",
      message: "page ready",
    });
  });

  it("keeps an unrecognized line as an unknown raw message", () => {
    const entry = DeviceLogEntry.parse("plain boot message\r\n");

    expect(entry).toMatchObject({
      raw: "plain boot message",
      timestamp: null,
      pid: null,
      tid: null,
      level: "unknown",
      domain: "",
      tag: "",
      process: "",
      message: "plain boot message",
    });
  });
});

import { describe, expect, it } from "vitest";
import { DeviceFaultLogEntry } from "../src/device-fault-log-entry";

describe("DeviceFaultLogEntry", () => {
  it("classifies an explicit JS error reason", () => {
    const entry = DeviceFaultLogEntry.parse({
      id: "fault-1",
      raw: "Reason: JS_ERROR\nSummary: width is undefined",
    });

    expect(entry.type).toBe("jsCrash");
  });

  it("classifies only the supported explicit reason taxonomy", () => {
    const types = [
      ["APP_CRASH", "cppCrash"],
      ["APP_FREEZE", "appFreeze"],
      ["APP_KILLED", "appKilled"],
      ["SYS_WARNING", "sysWarning"],
      ["a prose warning about a frozen UI", "unknown"],
    ].map(([reason]) => DeviceFaultLogEntry.parse({
      id: reason,
      raw: `Reason: ${reason}`,
    }).type);

    expect(types).toEqual(["cppCrash", "appFreeze", "appKilled", "sysWarning", "unknown"]);
  });

  it("formats a stable multiline summary", () => {
    const entry = DeviceFaultLogEntry.parse({
      id: "fault-summary",
      raw: [
        "Reason: APP_CRASH",
        "Process: com.example.camera",
        "PID: 2468",
        "Summary: Native crash in camera pipeline",
      ].join("\n"),
    });

    expect(entry.copySummary).toBe([
      "APP_CRASH: Native crash in camera pipeline",
      "Process: com.example.camera",
      "PID: 2468",
    ].join("\n"));
  });
});

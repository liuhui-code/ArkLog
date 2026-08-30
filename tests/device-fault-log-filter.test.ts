import { describe, expect, it } from "vitest";
import { DeviceFaultLogEntry } from "../src/device-fault-log-entry";
import {
  DeviceFaultLogFilter,
  EMPTY_DEVICE_FAULT_LOG_FILTER,
} from "../src/device-fault-log-filter";

const entry = DeviceFaultLogEntry.parse({
  id: "fault-1",
  raw: [
    "Reason: APP_FREEZE",
    "Process: com.example.camera",
    "PID: 987",
    "Summary: Main thread blocked by image decode",
  ].join("\n"),
});

describe("DeviceFaultLogFilter", () => {
  it("combines type, exact PID, process, and plain text", () => {
    const filter = new DeviceFaultLogFilter({
      ...EMPTY_DEVICE_FAULT_LOG_FILTER,
      type: "appFreeze",
      pid: "987",
      process: "CAMERA",
      query: "image decode",
    });

    expect(filter.matches(entry)).toBe(true);
    expect(new DeviceFaultLogFilter({
      ...EMPTY_DEVICE_FAULT_LOG_FILTER,
      type: "jsCrash",
    }).matches(entry)).toBe(false);
  });

  it("reports an invalid regular expression without matching entries", () => {
    const filter = new DeviceFaultLogFilter({
      ...EMPTY_DEVICE_FAULT_LOG_FILTER,
      query: "(",
      regex: true,
    });

    expect(filter.error).toMatch(/Invalid regular expression/u);
    expect(filter.matches(entry)).toBe(false);
  });

  it("reports a non-numeric PID without matching entries", () => {
    const filter = new DeviceFaultLogFilter({
      ...EMPTY_DEVICE_FAULT_LOG_FILTER,
      pid: "98x",
    });

    expect(filter.error).toBe("PID filter must be a number");
    expect(filter.matches(entry)).toBe(false);
  });

  it("skips regex evaluation for oversized raw entries", () => {
    const oversized = DeviceFaultLogEntry.parse({
      id: "large-fault",
      raw: `${"a".repeat(4_097)} target`,
    });
    const filter = new DeviceFaultLogFilter({
      ...EMPTY_DEVICE_FAULT_LOG_FILTER,
      query: "target$",
      regex: true,
    });

    expect(filter.matches(oversized)).toBe(false);
  });

  it("escapes plain metacharacters and applies case sensitivity to regex", () => {
    const literal = DeviceFaultLogEntry.parse({ id: "literal", raw: "Summary: state [ready]" });
    const plain = new DeviceFaultLogFilter({
      ...EMPTY_DEVICE_FAULT_LOG_FILTER,
      query: "[ready]",
    });
    const sensitive = new DeviceFaultLogFilter({
      ...EMPTY_DEVICE_FAULT_LOG_FILTER,
      query: "app_freeze",
      regex: true,
      matchCase: true,
    });

    expect(plain.matches(literal)).toBe(true);
    expect(sensitive.matches(entry)).toBe(false);
  });
});

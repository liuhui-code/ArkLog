import type { DeviceFaultLogRawEntry } from "./arklog-api";

export type DeviceFaultLogType =
  | "jsCrash"
  | "cppCrash"
  | "appFreeze"
  | "appKilled"
  | "sysWarning"
  | "unknown";

export class DeviceFaultLogEntry {
  readonly id: string;
  readonly raw: string;
  readonly reason: string;
  readonly summary: string;
  readonly processName: string;
  readonly pid: string;
  readonly timestamp: string;
  readonly type: DeviceFaultLogType;

  private constructor(entry: DeviceFaultLogRawEntry, fields: Map<string, string>) {
    this.id = entry.id;
    this.raw = entry.raw;
    this.reason = fields.get("reason") ?? "Unknown fault";
    this.summary = fields.get("summary") ?? firstLine(entry.raw);
    this.processName = fields.get("process") ?? "Unknown process";
    this.pid = fields.get("pid") ?? "—";
    this.timestamp = fields.get("timestamp") ?? "Unknown time";
    this.type = classifyReason(this.reason);
  }

  static parse(entry: DeviceFaultLogRawEntry) {
    return new DeviceFaultLogEntry(entry, parseFields(entry.raw));
  }

  get copySummary() {
    return [
      `${this.reason}: ${this.summary}`,
      `Process: ${this.processName}`,
      `PID: ${this.pid}`,
    ].join("\n");
  }
}

function classifyReason(reason: string): DeviceFaultLogType {
  switch (reason.trim().toUpperCase()) {
    case "JS_ERROR": return "jsCrash";
    case "APP_CRASH": return "cppCrash";
    case "APP_FREEZE": return "appFreeze";
    case "APP_KILLED": return "appKilled";
    case "SYS_WARNING": return "sysWarning";
    default: return "unknown";
  }
}

function parseFields(raw: string) {
  const fields = new Map<string, string>();
  for (const line of raw.replace(/\r\n/gu, "\n").split("\n")) {
    const match = /^([A-Za-z][A-Za-z0-9_]*):\s?(.*)$/u.exec(line);
    if (match) fields.set(match[1].toLowerCase(), match[2].trim());
  }
  return fields;
}

function firstLine(raw: string) {
  return raw.split(/\r?\n/u).find((line) => line.trim().length > 0)?.trim() ?? "Unknown fault";
}

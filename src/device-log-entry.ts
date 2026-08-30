export type DeviceLogLevel = "verbose" | "debug" | "info" | "warn" | "error" | "fatal" | "unknown";

const LEVELS: Record<string, DeviceLogLevel> = {
  V: "verbose",
  D: "debug",
  I: "info",
  W: "warn",
  E: "error",
  F: "fatal",
};

const HILOG_PATTERN = /^(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})\s+(\d+)\s+(\d+)\s+([VDIWEF])\s+([^/\s]+)\/([^\s]+)\s+([^:]+):\s?(.*)$/u;

export class DeviceLogEntry {
  private constructor(
    readonly raw: string,
    readonly timestamp: string | null,
    readonly level: DeviceLogLevel,
    readonly pid: number | null,
    readonly tid: number | null,
    readonly process: string,
    readonly domain: string,
    readonly tag: string,
    readonly message: string,
  ) {}

  static parse(raw: string) {
    const line = raw.replace(/\r?\n$/u, "");
    const match = HILOG_PATTERN.exec(line);
    return new DeviceLogEntry(
      line,
      match ? match[1] : null,
      match ? LEVELS[match[4]] ?? "unknown" : "unknown",
      match ? Number.parseInt(match[2], 10) : null,
      match ? Number.parseInt(match[3], 10) : null,
      match ? match[7].trim() : "",
      match ? match[5] : "",
      match ? match[6] : "",
      match ? match[8] : line,
    );
  }
}

import type { DeviceFaultLogEntry, DeviceFaultLogType } from "./device-fault-log-entry";

const REGEX_INPUT_LIMIT = 4_096;

export type DeviceFaultLogFilterState = Readonly<{
  query: string;
  regex: boolean;
  matchCase: boolean;
  type: "all" | DeviceFaultLogType;
  process: string;
  pid: string;
}>;

export const EMPTY_DEVICE_FAULT_LOG_FILTER: DeviceFaultLogFilterState = {
  query: "",
  regex: false,
  matchCase: false,
  type: "all",
  process: "",
  pid: "",
};

export class DeviceFaultLogFilter {
  private readonly pattern: RegExp | null;
  private readonly valid: boolean;
  readonly error: string | null;

  constructor(private readonly state: DeviceFaultLogFilterState) {
    const pid = state.pid.trim();
    if (pid && !/^\d+$/u.test(pid)) {
      this.pattern = null;
      this.valid = false;
      this.error = "PID filter must be a number";
      return;
    }
    const query = state.query.trim();
    if (!query) {
      this.pattern = null;
      this.valid = true;
      this.error = null;
      return;
    }
    try {
      this.pattern = new RegExp(
        state.regex ? query : escapeRegExp(query),
        state.matchCase ? "u" : "iu",
      );
      this.valid = true;
      this.error = null;
    } catch (error) {
      this.pattern = null;
      this.valid = false;
      this.error = error instanceof Error ? error.message : "Invalid regular expression";
    }
  }

  matches(entry: DeviceFaultLogEntry) {
    if (!this.valid) return false;
    if (this.state.regex && entry.raw.length > REGEX_INPUT_LIMIT) return false;
    if (this.state.type !== "all" && entry.type !== this.state.type) return false;
    if (this.state.pid.trim() && entry.pid !== this.state.pid.trim()) return false;
    if (!includesField(entry.processName, this.state.process, this.state.matchCase)) return false;
    return this.pattern === null || this.pattern.test(entry.raw);
  }
}

function includesField(value: string, query: string, matchCase: boolean) {
  const needle = query.trim();
  if (!needle) return true;
  return matchCase
    ? value.includes(needle)
    : value.toLocaleLowerCase().includes(needle.toLocaleLowerCase());
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

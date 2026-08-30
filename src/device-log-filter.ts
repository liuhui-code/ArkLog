import type { DeviceLogEntry } from "./device-log-entry";

export type DeviceLogFilterState = Readonly<{
  query: string;
}>;

export const EMPTY_DEVICE_LOG_FILTER: DeviceLogFilterState = {
  query: "",
};

export type DeviceLogMatchRange = Readonly<{
  start: number;
  end: number;
}>;

export class DeviceLogFilter {
  private readonly pattern: RegExp | null;
  private readonly globalPattern: RegExp | null;
  private readonly valid: boolean;
  readonly error: string | null;

  constructor(private readonly state: DeviceLogFilterState) {
    if (state.query.length === 0) {
      this.pattern = null;
      this.globalPattern = null;
      this.valid = true;
      this.error = null;
      return;
    }

    try {
      this.pattern = new RegExp(state.query, "u");
      this.globalPattern = new RegExp(state.query, "gu");
      this.valid = true;
      this.error = null;
    } catch (error) {
      this.pattern = null;
      this.globalPattern = null;
      this.valid = false;
      this.error = error instanceof Error ? error.message : "Invalid regular expression";
    }
  }

  matches(entry: DeviceLogEntry) {
    if (!this.valid) return false;
    return this.pattern === null || this.pattern.test(entry.raw);
  }

  matchRanges(entry: DeviceLogEntry): readonly DeviceLogMatchRange[] {
    if (!this.valid || this.globalPattern === null) return [];
    return Array.from(entry.raw.matchAll(this.globalPattern))
      .filter((match) => match[0].length > 0)
      .map((match) => ({
        start: match.index,
        end: match.index + match[0].length,
      }));
  }
}

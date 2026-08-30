import { DeviceLogEntry } from "./device-log-entry";
import { DeviceLogFind } from "./device-log-find";
import {
  DeviceLogFilter,
  EMPTY_DEVICE_LOG_FILTER,
  type DeviceLogFilterState,
} from "./device-log-filter";

export type DeviceLogSessionSnapshot = Readonly<{
  version: number;
  totalCount: number;
  visibleCount: number;
}>;

type FrameScheduler = (notify: () => void) => () => void;

type DeviceLogSessionOptions = {
  schedule?: FrameScheduler;
};

export class DeviceLogSession {
  private readonly entries: DeviceLogEntry[] = [];
  private readonly visibleIndices: number[] = [];
  private readonly listeners = new Set<() => void>();
  private readonly schedule: FrameScheduler;
  private filterState: DeviceLogFilterState = EMPTY_DEVICE_LOG_FILTER;
  private filter = new DeviceLogFilter(EMPTY_DEVICE_LOG_FILTER);
  private activeFind = new DeviceLogFind("", []);
  private findValid = true;
  private snapshot: DeviceLogSessionSnapshot = Object.freeze({
    version: 0,
    totalCount: 0,
    visibleCount: 0,
  });
  private cancelNotification: (() => void) | null = null;
  private disposed = false;

  constructor(options: DeviceLogSessionOptions = {}) {
    this.schedule = options.schedule ?? scheduleNextFrame;
  }

  readonly subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  readonly getSnapshot = () => this.snapshot;

  append(rawLines: readonly string[]) {
    if (this.disposed || rawLines.length === 0) return;
    const firstNewVisibleIndex = this.visibleIndices.length;
    const newVisibleLines: string[] = [];
    rawLines.forEach((raw) => {
      const entry = DeviceLogEntry.parse(raw);
      const index = this.entries.length;
      this.entries.push(entry);
      if (this.filter.matches(entry)) {
        this.visibleIndices.push(index);
        newVisibleLines.push(raw);
      }
    });
    if (this.findValid) this.activeFind.append(newVisibleLines, firstNewVisibleIndex);
    this.updateSnapshot();
  }

  setFilter(state: DeviceLogFilterState) {
    if (this.disposed || state.query === this.filterState.query) return;
    this.filterState = state;
    this.filter = new DeviceLogFilter(state);
    this.visibleIndices.length = 0;
    this.entries.forEach((entry, index) => {
      if (this.filter.matches(entry)) this.visibleIndices.push(index);
    });
    this.findValid = false;
    this.updateSnapshot();
  }

  clear() {
    if (this.disposed || this.entries.length === 0) return;
    this.entries.length = 0;
    this.visibleIndices.length = 0;
    this.findValid = false;
    this.updateSnapshot();
  }

  visibleEntries(start: number, end: number) {
    return this.visibleIndices
      .slice(Math.max(0, start), Math.max(0, end))
      .map((index) => this.entries[index]);
  }

  visibleRawLines() {
    return this.visibleIndices.map((index) => this.entries[index].raw);
  }

  find(query: string) {
    if (!this.findValid || query !== this.activeFind.query) {
      this.activeFind = new DeviceLogFind(query, this.visibleRawLines());
      this.findValid = true;
    }
    return this.activeFind;
  }

  dispose() {
    this.disposed = true;
    this.cancelNotification?.();
    this.cancelNotification = null;
    this.listeners.clear();
  }

  private updateSnapshot() {
    this.snapshot = Object.freeze({
      version: this.snapshot.version + 1,
      totalCount: this.entries.length,
      visibleCount: this.visibleIndices.length,
    });
    this.scheduleNotification();
  }

  private scheduleNotification() {
    if (this.cancelNotification) return;
    let completedSynchronously = false;
    const cancel = this.schedule(() => {
      completedSynchronously = true;
      this.cancelNotification = null;
      this.listeners.forEach((listener) => listener());
    });
    if (!completedSynchronously) this.cancelNotification = cancel;
  }
}

function scheduleNextFrame(notify: () => void) {
  if (typeof requestAnimationFrame === "function") {
    const frame = requestAnimationFrame(notify);
    return () => cancelAnimationFrame(frame);
  }
  notify();
  return () => {};
}

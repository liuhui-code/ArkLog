import { describe, expect, it } from "vitest";
import { DeviceLogSession } from "../src/device-log-session";

describe("DeviceLogSession", () => {
  it("retains 100,000 ordered lines while coalescing repaint notifications", () => {
    const scheduled: Array<() => void> = [];
    const session = new DeviceLogSession({
      schedule: (notify) => {
        scheduled.push(notify);
        return () => {};
      },
    });
    let notifications = 0;
    session.subscribe(() => { notifications += 1; });

    session.append(Array.from({ length: 50_000 }, (_, index) => `raw ${index}`));
    session.append(Array.from({ length: 50_000 }, (_, index) => `raw ${index + 50_000}`));

    expect(session.getSnapshot()).toEqual({ version: 2, totalCount: 100_000, visibleCount: 100_000 });
    expect(session.getSnapshot()).toBe(session.getSnapshot());
    expect(session.visibleEntries(49_999, 50_002).map((entry) => entry.raw)).toEqual([
      "raw 49999",
      "raw 50000",
      "raw 50001",
    ]);
    expect(scheduled).toHaveLength(1);
    expect(notifications).toBe(0);

    scheduled[0]();

    expect(notifications).toBe(1);
  });

  it("rebuilds a changed filter and incrementally indexes later batches", () => {
    const scheduled: Array<() => void> = [];
    const session = new DeviceLogSession({
      schedule: (notify) => {
        scheduled.push(notify);
        return () => {};
      },
    });
    session.append(["keep old", "drop old"]);
    scheduled.shift()?.();

    session.setFilter({ query: "^keep" });
    session.append(["drop new", "keep new"]);

    expect(session.getSnapshot()).toEqual({ version: 3, totalCount: 4, visibleCount: 2 });
    expect(session.visibleEntries(0, 10).map((entry) => entry.raw)).toEqual([
      "keep old",
      "keep new",
    ]);
    expect(scheduled).toHaveLength(1);
  });

  it("clears pending data without losing lines delivered after clear", () => {
    const scheduled: Array<() => void> = [];
    const session = new DeviceLogSession({
      schedule: (notify) => {
        scheduled.push(notify);
        return () => {};
      },
    });
    let observed: string[] = [];
    session.subscribe(() => {
      observed = session.visibleEntries(0, 10).map((entry) => entry.raw);
    });

    session.append(["before clear"]);
    const beforeEmpty = session.getSnapshot();
    session.append([]);
    expect(session.getSnapshot()).toBe(beforeEmpty);
    session.clear();
    session.append(["after clear"]);

    expect(scheduled).toHaveLength(1);
    expect(session.getSnapshot()).toEqual({ version: 3, totalCount: 1, visibleCount: 1 });
    scheduled[0]();
    expect(observed).toEqual(["after clear"]);
  });

  it("increments an active find index and rebuilds it after filter or clear", () => {
    const session = new DeviceLogSession();
    session.append(["needle old", "ignored"]);

    const initial = session.find("needle");
    expect(initial.count).toBe(1);
    session.append(["needle new", "other"]);

    expect(session.find("needle")).toBe(initial);
    expect(initial.occurrences.map((match) => match.lineIndex)).toEqual([0, 2]);

    session.setFilter({ query: "^needle" });
    expect(session.find("needle").occurrences.map((match) => match.lineIndex)).toEqual([0, 1]);

    session.clear();
    session.append(["needle after clear"]);
    expect(session.find("needle").occurrences.map((match) => match.lineIndex)).toEqual([0]);
  });
});

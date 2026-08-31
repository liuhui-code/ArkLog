import { describe, expect, it } from "vitest";
import { createTauriArkLogApi } from "../src/tauri-api";

describe("Tauri ArkLog API", () => {
  it("exposes no persisted HiLog history boundary", () => {
    const api = createTauriArkLogApi({
      invoke: async <T>() => undefined as T,
      listen: async () => () => undefined,
    });

    expect(api).not.toHaveProperty("queryHistory");
    expect(api).not.toHaveProperty("getStorageHealth");
    expect(api).not.toHaveProperty("clearHistory");
  });

  it("starts a stream and forwards its output event through the public subscription", async () => {
    const boundary = {
      eventHandler: null as ((event: { payload: unknown }) => void) | null,
    };
    const api = createTauriArkLogApi({
      invoke: async <T>(command: string, args?: Record<string, unknown>) => {
        if (command !== "start_device_log_stream" || args?.deviceId !== "USB-01") {
          throw new Error(`unexpected invoke: ${command}`);
        }
        return { streamId: "stream-1", deviceId: "USB-01", status: "running" } as T;
      },
      listen: async (_event, handler) => {
        boundary.eventHandler = handler as (event: { payload: unknown }) => void;
        return () => undefined;
      },
    });
    const received: string[] = [];

    const stream = await api.startStream("USB-01");
    await api.subscribeToOutput((batch) => received.push(...batch.lines));
    boundary.eventHandler?.({
      payload: { streamId: stream.streamId, deviceId: "USB-01", lines: ["ready"] },
    });

    expect(received).toEqual(["ready"]);
  });

  it("fetches fault logs for the selected device through the Tauri boundary", async () => {
    const api = createTauriArkLogApi({
      invoke: async <T>(command: string, args?: Record<string, unknown>) => {
        if (command !== "list_device_fault_logs" || args?.deviceId !== "USB-01") {
          throw new Error(`unexpected invoke: ${command}`);
        }
        return {
          deviceId: "USB-01",
          entries: [{ id: "USB-01-fault-1", raw: "Reason: JS_ERROR" }],
          command: 'hdc -t USB-01 shell hidumper -s 1201 -a "-p Faultlogger -l -d"',
          stderr: "",
          status: "ready",
          message: "ok",
        } as T;
      },
      listen: async () => () => undefined,
    });

    const result = await api.listFaultLogs("USB-01");

    expect(result.status).toBe("ready");
    expect(result.entries[0].raw).toContain("JS_ERROR");
  });

  it("forwards copied text through the clipboard boundary", async () => {
    const copied: string[] = [];
    const api = createTauriArkLogApi({
      invoke: async <T>() => undefined as T,
      listen: async () => () => undefined,
      writeClipboard: async (text) => { copied.push(text); },
    });

    await api.writeClipboard("fault summary");

    expect(copied).toEqual(["fault summary"]);
  });

  it("exports selected fault entries through one Tauri command", async () => {
    const entries = [{ id: "fault-1", raw: "Reason: JS_ERROR" }];
    const api = createTauriArkLogApi({
      invoke: async <T>(command: string, args?: Record<string, unknown>) => {
        if (command === "export_device_fault_logs" && args?.entries instanceof Array && args.entries.length === 0) {
          return null as T;
        }
        if (
          command !== "export_device_fault_logs"
          || args?.deviceId !== "USB-01"
          || args?.entries !== entries
        ) {
          throw new Error(`unexpected invoke: ${command}`);
        }
        return { path: "/tmp/faults.txt", entryCount: 1, bytesWritten: 80 } as T;
      },
      listen: async () => () => undefined,
    });

    const result = await api.exportFaultLogs("USB-01", entries);
    const cancelled = await api.exportFaultLogs("USB-01", []);

    expect(result).toEqual({ path: "/tmp/faults.txt", entryCount: 1, bytesWritten: 80 });
    expect(cancelled).toBeNull();
  });
});

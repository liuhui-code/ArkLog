import { describe, expect, it } from "vitest";
import { createTauriArkLogApi } from "../src/tauri-api";

describe("Tauri ArkLog API", () => {
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
});

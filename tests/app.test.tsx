import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../src/App";
import type { ArkLogApi, DeviceLogOutputBatch } from "../src/arklog-api";

describe("ArkLog workbench", () => {
  it("starts the selected device stream and renders incoming HiLog lines", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    expect(await screen.findByRole("option", { name: "USB-01 · online" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    expect(await screen.findByText("Streaming USB-01")).toBeVisible();

    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["08-28 09:31:02.441 1042 1042 I ArkUI: page mounted"],
    }));

    expect(screen.getByText(/page mounted/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Stop stream" }));
    expect(await screen.findByText("Stopped")).toBeVisible();
  });
});

class FakeArkLogApi implements ArkLogApi {
  private listener: ((batch: DeviceLogOutputBatch) => void) | null = null;

  async listDevices() {
    return [{ id: "USB-01", label: "USB-01", status: "online" as const, detail: "Connected" }];
  }

  async startStream(deviceId: string) {
    return { streamId: "stream-1", deviceId, status: "running" as const };
  }

  async stopStream() {}

  async subscribeToOutput(listener: (batch: DeviceLogOutputBatch) => void) {
    this.listener = listener;
    return () => { this.listener = null; };
  }

  emit(batch: DeviceLogOutputBatch) {
    this.listener?.(batch);
  }
}

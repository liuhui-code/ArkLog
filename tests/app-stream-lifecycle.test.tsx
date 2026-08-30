import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../src/App";
import { FakeArkLogApi } from "./support/fake-arklog-api";

afterEach(cleanup);

describe("ArkLog stream lifecycle", () => {
  it("retains the first batch emitted before start returns and rejects another stream", async () => {
    const api = new EarlyBatchArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    expect(await screen.findByText("Streaming USB-01")).toBeVisible();

    await waitFor(() => {
      expect(queryLogLine("first during start")).toBeVisible();
    });
    expect(queryLogLine("stale during start")).not.toBeInTheDocument();
    expect(screen.getByText("1")).toBeVisible();
  });

  it("does not start until the output subscription is ready", async () => {
    const api = new DelayedSubscriptionArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    const start = screen.getByRole("button", { name: "Start stream" });
    expect(start).toBeDisabled();

    api.connectOutput();
    await waitFor(() => expect(start).toBeEnabled());
    await user.click(start);

    expect(await screen.findByText("Streaming USB-01")).toBeVisible();
  });

  it("renders a final batch delivered while stop is completing", async () => {
    const api = new TailOnStopArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);
    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    await user.click(await screen.findByRole("button", { name: "Stop stream" }));

    expect(await screen.findByText("Stopped")).toBeVisible();
    await waitFor(() => expect(queryLogLine("tail during stop")).toBeVisible());
  });

  it("prevents a second start while the first start request is pending", async () => {
    const api = new DelayedStartArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);
    await screen.findByRole("option", { name: "USB-01 · online" });
    const start = screen.getByRole("button", { name: "Start stream" });
    await user.click(start);

    expect(start).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Device" })).toBeDisabled();
    expect(api.startCalls).toBe(1);
    api.finishStart();
    expect(await screen.findByText("Streaming USB-01")).toBeVisible();
  });

  it("prevents a second stop while the first stop request is pending", async () => {
    const api = new DelayedStopArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);
    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    const stop = await screen.findByRole("button", { name: "Stop stream" });
    await user.click(stop);

    expect(stop).toBeDisabled();
    expect(api.stopCalls).toBe(1);
    api.finishStop();
    expect(await screen.findByText("Stopped")).toBeVisible();
  });
});

class EarlyBatchArkLogApi extends FakeArkLogApi {
  override async startStream(deviceId: string) {
    this.emit({
      streamId: "stream-1",
      deviceId,
      lines: ["first during start"],
    });
    this.emit({
      streamId: "stale-stream",
      deviceId,
      lines: ["stale during start"],
    });
    return super.startStream(deviceId);
  }
}

class DelayedSubscriptionArkLogApi extends FakeArkLogApi {
  private connect: (() => void) | null = null;

  override subscribeToOutput(listener: Parameters<FakeArkLogApi["subscribeToOutput"]>[0]) {
    return new Promise<() => void>((resolve) => {
      this.connect = () => {
        void super.subscribeToOutput(listener).then(resolve);
      };
    });
  }

  connectOutput() {
    this.connect?.();
  }
}

class TailOnStopArkLogApi extends FakeArkLogApi {
  override async stopStream() {
    this.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["tail during stop"],
    });
  }
}

class DelayedStartArkLogApi extends FakeArkLogApi {
  startCalls = 0;
  private finish: (() => void) | null = null;

  override async startStream(deviceId: string) {
    this.startCalls += 1;
    await new Promise<void>((resolve) => { this.finish = resolve; });
    return super.startStream(deviceId);
  }

  finishStart() {
    this.finish?.();
  }
}

class DelayedStopArkLogApi extends FakeArkLogApi {
  stopCalls = 0;
  private finish: (() => void) | null = null;

  override async stopStream() {
    this.stopCalls += 1;
    await new Promise<void>((resolve) => { this.finish = resolve; });
  }

  finishStop() {
    this.finish?.();
  }
}

function queryLogLine(raw: string) {
  const viewport = screen.queryByRole("region", { name: "HiLog output" });
  if (!viewport) return null;
  return [...viewport.querySelectorAll("code")]
    .find((element) => element.textContent === raw) ?? null;
}

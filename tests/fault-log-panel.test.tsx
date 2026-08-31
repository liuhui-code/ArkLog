import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { DeviceFaultLogPanel } from "../src/DeviceFaultLogPanel";
import type {
  ArkLogApi,
  DeviceFaultLogExportResult,
  DeviceFaultLogFetchResult,
  DeviceFaultLogRawEntry,
  DeviceLogOutputBatch,
} from "../src/arklog-api";

afterEach(cleanup);

describe("DeviceFaultLogPanel", () => {
  it("combines type, process, PID, and text filters", async () => {
    const api = new FaultLogApi();
    const user = userEvent.setup();
    render(<DeviceFaultLogPanel api={api} deviceId="USB-01" hidden={false} onStatusChange={() => undefined} />);

    await user.click(screen.getByRole("button", { name: "Refresh Fault Logs" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Fault log type" }), "appFreeze");
    fireEvent.change(screen.getByRole("textbox", { name: "Fault log process" }), { target: { value: "CAMERA" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Fault log PID" }), { target: { value: "987" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Filter fault logs" }), { target: { value: "image decode" } });

    expect(screen.getByRole("button", { name: /Main thread blocked/u })).toBeVisible();
    expect(screen.queryByRole("button", { name: /width is undefined/u })).not.toBeInTheDocument();
    expect(screen.getByLabelText("Fault Log Inspector")).toHaveTextContent("APP_FREEZE");
  });

  it("copies the visible selection summary and exact raw payload", async () => {
    const api = new FaultLogApi();
    const user = userEvent.setup();
    render(<DeviceFaultLogPanel api={api} deviceId="USB-01" hidden={false} onStatusChange={() => undefined} />);

    await user.click(screen.getByRole("button", { name: "Refresh Fault Logs" }));
    await user.click(screen.getByRole("button", { name: /Main thread blocked/u }));
    await user.click(screen.getByRole("button", { name: "Copy Fault Summary" }));
    await user.click(screen.getByRole("button", { name: "Copy Fault Raw" }));

    expect(api.copied).toEqual([
      [
        "APP_FREEZE: Main thread blocked by image decode",
        "Process: com.example.camera",
        "PID: 987",
      ].join("\n"),
      api.result.entries[1].raw,
    ]);
  });

  it("reports invalid regex and falls back to the first visible selection", async () => {
    const api = new FaultLogApi();
    const user = userEvent.setup();
    render(<DeviceFaultLogPanel api={api} deviceId="USB-01" hidden={false} onStatusChange={() => undefined} />);

    await user.click(screen.getByRole("button", { name: "Refresh Fault Logs" }));
    await user.click(screen.getByRole("button", { name: /Main thread blocked/u }));
    await user.click(screen.getByRole("checkbox", { name: "Regex" }));
    const query = screen.getByRole("textbox", { name: "Filter fault logs" });
    await user.type(query, "(");

    expect(screen.getByRole("alert")).toHaveTextContent(/Invalid regular expression/u);
    await user.clear(query);
    await user.type(query, "width");
    expect(screen.getByLabelText("Fault Log Inspector")).toHaveTextContent("JS_ERROR");
  });

  it("exports only fault entries visible through the current filters", async () => {
    const api = new FaultLogApi();
    const statuses: string[] = [];
    const user = userEvent.setup();
    render(
      <DeviceFaultLogPanel
        api={api}
        deviceId="USB-01"
        hidden={false}
        onStatusChange={(message) => statuses.push(message)}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Refresh Fault Logs" }));
    await user.type(screen.getByRole("textbox", { name: "Filter fault logs" }), "width");
    await user.click(screen.getByRole("button", { name: "Export Visible Fault Logs" }));

    expect(api.exported).toEqual([{ deviceId: "USB-01", entries: [api.result.entries[0]] }]);
    expect(statuses.at(-1)).toBe("Exported 1 fault log to /tmp/visible-faults.txt");
    expect(screen.getByLabelText("Fault Log Inspector")).toHaveTextContent("JS_ERROR");
  });

  it("treats cancelling the save dialog as a normal outcome", async () => {
    const api = new FaultLogApi();
    api.exportResult = null;
    const statuses: string[] = [];
    const user = userEvent.setup();
    render(
      <DeviceFaultLogPanel
        api={api}
        deviceId="USB-01"
        hidden={false}
        onStatusChange={(message) => statuses.push(message)}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Refresh Fault Logs" }));
    await user.click(screen.getByRole("button", { name: "Export Visible Fault Logs" }));

    expect(api.exported).toHaveLength(1);
    expect(statuses.at(-1)).toBe("Fault log export cancelled");
  });
});

class FaultLogApi implements ArkLogApi {
  copied: string[] = [];
  exported: Array<{ deviceId: string; entries: DeviceFaultLogRawEntry[] }> = [];
  exportResult: DeviceFaultLogExportResult | null = {
    path: "/tmp/visible-faults.txt",
    entryCount: 1,
    bytesWritten: 120,
  };
  result: DeviceFaultLogFetchResult = {
    deviceId: "USB-01",
    entries: [
      {
        id: "fault-js",
        raw: "Reason: JS_ERROR\nProcess: com.example.alpha\nPID: 101\nSummary: width is undefined",
      },
      {
        id: "fault-freeze",
        raw: "Reason: APP_FREEZE\nProcess: com.example.camera\nPID: 987\nSummary: Main thread blocked by image decode",
      },
    ],
    command: 'hdc -t USB-01 shell hidumper -s 1201 -a "-p Faultlogger -l -d"',
    stderr: "",
    status: "ready",
    message: "ok",
  };

  async listDevices() { return []; }
  async startStream(deviceId: string) {
    return { streamId: "unused", deviceId, status: "running" as const };
  }
  async stopStream() {}
  async listFaultLogs() { return this.result; }
  async exportFaultLogs(deviceId: string, entries: DeviceFaultLogRawEntry[]) {
    this.exported.push({ deviceId, entries });
    return this.exportResult;
  }
  async writeClipboard(text: string) { this.copied.push(text); }
  async subscribeToOutput(_listener: (batch: DeviceLogOutputBatch) => void) {
    return () => undefined;
  }
}

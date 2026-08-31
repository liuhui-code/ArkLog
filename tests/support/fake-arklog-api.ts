import type {
  ArkLogApi,
  DeviceFaultLogFetchResult,
  DeviceLogOutputBatch,
} from "../../src/arklog-api";

export class FakeArkLogApi implements ArkLogApi {
  private listener: ((batch: DeviceLogOutputBatch) => void) | null = null;
  devices = [{ id: "USB-01", label: "USB-01", status: "online" as const, detail: "Connected" }];
  faultLogDevices: string[] = [];
  faultLogResult: DeviceFaultLogFetchResult = {
    deviceId: "USB-01",
    entries: [],
    command: 'hdc -t USB-01 shell hidumper -s 1201 -a "-p Faultlogger -l -d"',
    stderr: "",
    status: "empty",
    message: "No fault logs found",
  };

  async listDevices() {
    return this.devices;
  }

  async startStream(deviceId: string) {
    return { streamId: "stream-1", deviceId, status: "running" as const };
  }

  async stopStream() {}

  async listFaultLogs(deviceId: string) {
    this.faultLogDevices.push(deviceId);
    return this.faultLogResult;
  }

  async exportFaultLogs() { return null; }
  async writeClipboard() {}

  async subscribeToOutput(listener: (batch: DeviceLogOutputBatch) => void) {
    this.listener = listener;
    return () => { this.listener = null; };
  }

  emit(batch: DeviceLogOutputBatch) {
    this.listener?.(batch);
  }
}

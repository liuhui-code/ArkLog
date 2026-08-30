export type DeviceConnectionStatus = "unknown" | "online" | "offline" | "unauthorized";

export type DeviceLogDevice = {
  id: string;
  label: string;
  status: DeviceConnectionStatus;
  detail: string;
};

export type DeviceLogStreamSummary = {
  streamId: string;
  deviceId: string;
  status: "running";
};

export type DeviceLogOutputBatch = {
  streamId: string;
  deviceId: string;
  lines: string[];
};

export type DeviceFaultLogStatus =
  | "ready"
  | "empty"
  | "unavailable"
  | "unauthorized"
  | "error";

export type DeviceFaultLogRawEntry = {
  id: string;
  raw: string;
};

export type DeviceFaultLogFetchResult = {
  deviceId: string;
  entries: DeviceFaultLogRawEntry[];
  command: string;
  stderr: string;
  status: DeviceFaultLogStatus;
  message: string;
};

export type DeviceFaultLogExportResult = {
  path: string;
  entryCount: number;
  bytesWritten: number;
};

export type ArkLogApi = {
  listDevices(): Promise<DeviceLogDevice[]>;
  startStream(deviceId: string): Promise<DeviceLogStreamSummary>;
  stopStream(streamId: string): Promise<void>;
  listFaultLogs(deviceId: string): Promise<DeviceFaultLogFetchResult>;
  exportFaultLogs(
    deviceId: string,
    entries: DeviceFaultLogRawEntry[],
  ): Promise<DeviceFaultLogExportResult | null>;
  writeClipboard(text: string): Promise<void>;
  subscribeToOutput(listener: (batch: DeviceLogOutputBatch) => void): Promise<() => void>;
};

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

export type ArkLogApi = {
  listDevices(): Promise<DeviceLogDevice[]>;
  startStream(deviceId: string): Promise<DeviceLogStreamSummary>;
  stopStream(streamId: string): Promise<void>;
  subscribeToOutput(listener: (batch: DeviceLogOutputBatch) => void): Promise<() => void>;
};

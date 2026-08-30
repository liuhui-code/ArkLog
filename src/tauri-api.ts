import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type {
  ArkLogApi,
  DeviceFaultLogExportResult,
  DeviceFaultLogFetchResult,
  DeviceLogDevice,
  DeviceLogOutputBatch,
  DeviceLogStreamSummary,
} from "./arklog-api";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
type Listen = <T>(
  event: string,
  handler: (event: { payload: T }) => void,
) => Promise<() => void>;

type TauriBoundary = {
  invoke: Invoke;
  listen: Listen;
  writeClipboard?: (text: string) => Promise<void>;
};

const productionBoundary: TauriBoundary = {
  invoke: tauriInvoke,
  listen: tauriListen,
  writeClipboard: (text) => navigator.clipboard.writeText(text),
};

export function createTauriArkLogApi(
  boundary: TauriBoundary = productionBoundary,
): ArkLogApi {
  return {
    listDevices: () => boundary.invoke<DeviceLogDevice[]>("list_device_log_devices"),
    startStream: (deviceId) => boundary.invoke<DeviceLogStreamSummary>(
      "start_device_log_stream",
      { deviceId },
    ),
    stopStream: (streamId) => boundary.invoke<void>("stop_device_log_stream", { streamId }),
    listFaultLogs: (deviceId) => boundary.invoke<DeviceFaultLogFetchResult>(
      "list_device_fault_logs",
      { deviceId },
    ),
    exportFaultLogs: (deviceId, entries) => boundary.invoke<DeviceFaultLogExportResult | null>(
      "export_device_fault_logs",
      { deviceId, entries },
    ),
    writeClipboard: (text) => boundary.writeClipboard
      ? boundary.writeClipboard(text)
      : navigator.clipboard.writeText(text),
    subscribeToOutput: (listener) => boundary.listen<DeviceLogOutputBatch>(
      "device-log-output",
      (event) => listener(event.payload),
    ),
  };
}

export const tauriArkLogApi = createTauriArkLogApi();

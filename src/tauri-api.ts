import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type {
  ArkLogApi,
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
};

const productionBoundary: TauriBoundary = {
  invoke: tauriInvoke,
  listen: tauriListen,
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
    subscribeToOutput: (listener) => boundary.listen<DeviceLogOutputBatch>(
      "device-log-output",
      (event) => listener(event.payload),
    ),
  };
}

export const tauriArkLogApi = createTauriArkLogApi();

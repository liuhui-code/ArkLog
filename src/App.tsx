import { useEffect, useRef, useState } from "react";
import type { ArkLogApi, DeviceLogDevice } from "./arklog-api";

const MAX_VISIBLE_LINES = 2_000;

type AppProps = {
  api: ArkLogApi;
};

export function App({ api }: AppProps) {
  const [devices, setDevices] = useState<DeviceLogDevice[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState("");
  const [streamId, setStreamId] = useState<string | null>(null);
  const [status, setStatus] = useState("Discovering devices…");
  const [lines, setLines] = useState<string[]>([]);
  const activeStreamRef = useRef<string | null>(null);

  useEffect(() => {
    let active = true;
    void api.listDevices().then((items) => {
      if (!active) return;
      setDevices(items);
      setSelectedDeviceId(items.find((item) => item.status === "online")?.id ?? items[0]?.id ?? "");
      setStatus(items.length === 0 ? "No devices" : `${items.length} device${items.length === 1 ? "" : "s"} found`);
    }).catch((error: unknown) => {
      if (active) setStatus(error instanceof Error ? error.message : String(error));
    });
    return () => { active = false; };
  }, [api]);

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | null = null;
    void api.subscribeToOutput((batch) => {
      if (batch.streamId !== activeStreamRef.current) return;
      setLines((current) => [...current, ...batch.lines].slice(-MAX_VISIBLE_LINES));
    }).then((nextUnsubscribe) => {
      if (active) unsubscribe = nextUnsubscribe;
      else nextUnsubscribe();
    });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [api]);

  const selectedDevice = devices.find((device) => device.id === selectedDeviceId) ?? null;
  const isStreaming = streamId !== null;

  async function startStream() {
    if (!selectedDeviceId) return;
    setStatus("Starting stream…");
    try {
      const stream = await api.startStream(selectedDeviceId);
      activeStreamRef.current = stream.streamId;
      setStreamId(stream.streamId);
      setLines([]);
      setStatus(`Streaming ${stream.deviceId}`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function stopStream() {
    if (!streamId) return;
    setStatus("Stopping…");
    try {
      await api.stopStream(streamId);
      activeStreamRef.current = null;
      setStreamId(null);
      setStatus("Stopped");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <main className="workbench">
      <header className="app-header">
        <div className="brand-block">
          <span className="brand-mark" aria-hidden="true">AL</span>
          <div>
            <p className="eyebrow">HARMONYOS / DEVICE TELEMETRY</p>
            <h1>ArkLog</h1>
          </div>
        </div>
        <div className={`runtime-status ${isStreaming ? "is-live" : ""}`}>
          <span className="status-dot" aria-hidden="true" />
          <span>{status}</span>
        </div>
      </header>
      <section className="control-strip" aria-label="Stream controls">
        <label className="device-field">
          <span className="field-label">TARGET DEVICE</span>
          <select
            aria-label="Device"
            value={selectedDeviceId}
            disabled={streamId !== null}
            onChange={(event) => setSelectedDeviceId(event.target.value)}
          >
            {devices.length === 0 ? <option value="">No devices</option> : null}
            {devices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.label} · {device.status}
              </option>
            ))}
          </select>
        </label>
        <div className="device-readout" aria-label="Selected device details">
          <span className="field-label">CONNECTION</span>
          <strong>{selectedDevice?.status ?? "unavailable"}</strong>
          <small>{selectedDevice?.detail ?? "Waiting for HDC discovery"}</small>
        </div>
        {streamId ? (
          <button className="stream-button stop" type="button" aria-label="Stop stream" onClick={() => void stopStream()}>
            <span aria-hidden="true">■</span> Stop stream
          </button>
        ) : (
          <button
            className="stream-button start"
            type="button"
            aria-label="Start stream"
            disabled={!selectedDevice || selectedDevice.status !== "online"}
            onClick={() => void startStream()}
          >
            <span aria-hidden="true">▶</span> Start stream
          </button>
        )}
      </section>
      <section className="log-panel" aria-label="HiLog output">
        <div className="panel-heading">
          <div>
            <span className="panel-kicker">LIVE CHANNEL</span>
            <h2>HiLog output</h2>
          </div>
          <div className="line-meter">
            <strong>{lines.length.toLocaleString()}</strong>
            <span>lines buffered</span>
          </div>
        </div>
        <div className="log-viewport">
          {lines.length === 0 ? (
            <div className="empty-state">
              <span className="empty-glyph" aria-hidden="true">⌁</span>
              <p>Waiting for log output.</p>
              <small>Select an online device and start the stream.</small>
            </div>
          ) : (
          <ol className="log-lines">
            {lines.map((line, index) => (
              <li key={`${index}:${line}`} className={severityClass(line)}>
                <code>{line}</code>
              </li>
            ))}
          </ol>
          )}
        </div>
        <footer className="panel-footer">
          <span>BUFFER LIMIT {MAX_VISIBLE_LINES.toLocaleString()}</span>
          <span>{selectedDevice?.id ?? "NO TARGET"}</span>
        </footer>
      </section>
    </main>
  );
}

function severityClass(line: string) {
  const normalized = line.toLowerCase();
  if (normalized.includes(" error") || normalized.includes("fatal")) return "severity-error";
  if (normalized.includes(" warn")) return "severity-warning";
  return "";
}

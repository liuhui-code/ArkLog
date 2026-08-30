import { useEffect, useMemo, useRef, useState } from "react";
import type { ArkLogApi, DeviceFaultLogFetchResult } from "./arklog-api";
import { DeviceFaultLogEntry } from "./device-fault-log-entry";
import {
  DeviceFaultLogFilter,
  EMPTY_DEVICE_FAULT_LOG_FILTER,
  type DeviceFaultLogFilterState,
} from "./device-fault-log-filter";

type DeviceFaultLogPanelProps = {
  api: ArkLogApi;
  deviceId: string;
  hidden: boolean;
  onStatusChange: (message: string) => void;
};

export function DeviceFaultLogPanel({
  api,
  deviceId,
  hidden,
  onStatusChange,
}: DeviceFaultLogPanelProps) {
  const [result, setResult] = useState<DeviceFaultLogFetchResult | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [filterState, setFilterState] = useState(EMPTY_DEVICE_FAULT_LOG_FILTER);
  const requestId = useRef(0);
  const entries = useMemo(
    () => result?.entries.map(DeviceFaultLogEntry.parse) ?? [],
    [result],
  );
  const filter = new DeviceFaultLogFilter(filterState);
  const visibleEntries = entries.filter((entry) => filter.matches(entry));
  const selected = visibleEntries.find((entry) => entry.id === selectedId) ?? visibleEntries[0] ?? null;

  function updateFilter(patch: Partial<DeviceFaultLogFilterState>) {
    setFilterState((current) => ({ ...current, ...patch }));
  }

  useEffect(() => {
    requestId.current += 1;
    setResult(null);
    setSelectedId(null);
    setLoading(false);
  }, [deviceId]);

  async function refresh() {
    if (!deviceId || loading) return;
    const currentRequest = requestId.current + 1;
    requestId.current = currentRequest;
    setLoading(true);
    onStatusChange(`Refreshing fault logs for ${deviceId}…`);
    try {
      const next = await api.listFaultLogs(deviceId);
      if (requestId.current !== currentRequest) return;
      setResult(next);
      setSelectedId(next.entries[0]?.id ?? null);
      onStatusChange(
        next.status === "ready"
          ? `Loaded ${next.entries.length} fault log${next.entries.length === 1 ? "" : "s"}`
          : `${deviceId}: ${next.message}`,
      );
    } catch (error) {
      if (requestId.current !== currentRequest) return;
      const message = error instanceof Error ? error.message : String(error);
      setResult({
        deviceId,
        entries: [],
        command: "",
        stderr: "",
        status: "error",
        message,
      });
      onStatusChange(`${deviceId}: ${message}`);
    } finally {
      if (requestId.current === currentRequest) setLoading(false);
    }
  }

  async function copy(text: string, label: string) {
    try {
      await api.writeClipboard(text);
      onStatusChange(`Copied fault ${label}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      onStatusChange(`Failed to copy fault ${label}: ${message}`);
    }
  }

  async function exportVisible() {
    if (!deviceId || loading || exporting || visibleEntries.length === 0) return;
    const rawEntries = visibleEntries.map(({ id, raw }) => ({ id, raw }));
    setExporting(true);
    onStatusChange(`Exporting ${rawEntries.length} fault logs…`);
    try {
      const exportResult = await api.exportFaultLogs(deviceId, rawEntries);
      if (exportResult === null) {
        onStatusChange("Fault log export cancelled");
      } else {
        const noun = exportResult.entryCount === 1 ? "fault log" : "fault logs";
        onStatusChange(`Exported ${exportResult.entryCount} ${noun} to ${exportResult.path}`);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      onStatusChange(`Failed to export fault logs: ${message}`);
    } finally {
      setExporting(false);
    }
  }

  return (
    <div className="log-panel-view fault-log-panel" hidden={hidden}>
      <div className="panel-heading">
        <div>
          <span className="panel-kicker">DEVICE DIAGNOSTICS</span>
          <h2>Fault Log</h2>
        </div>
        <div className="panel-actions">
          <button
            type="button"
            className="panel-action-button"
            aria-label="Export Visible Fault Logs"
            disabled={!deviceId || loading || exporting || visibleEntries.length === 0}
            onClick={() => void exportVisible()}
          >
            {exporting ? "Exporting…" : "Export visible"}
          </button>
          <button
            type="button"
            className="panel-action-button"
            aria-label="Refresh Fault Logs"
            disabled={!deviceId || loading || exporting}
            onClick={() => void refresh()}
          >
            {loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>
      </div>
      <div className="fault-log-filter-bar">
        <select
          aria-label="Fault log type"
          value={filterState.type}
          onChange={(event) => updateFilter({ type: event.target.value as DeviceFaultLogFilterState["type"] })}
        >
          <option value="all">All types</option>
          <option value="jsCrash">JS crash</option>
          <option value="cppCrash">C++ crash</option>
          <option value="appFreeze">Freeze</option>
          <option value="appKilled">Killed</option>
          <option value="sysWarning">System warning</option>
          <option value="unknown">Unknown</option>
        </select>
        <input
          type="text"
          aria-label="Filter fault logs"
          placeholder="Filter fault logs"
          value={filterState.query}
          onChange={(event) => updateFilter({ query: event.target.value })}
        />
        <input
          type="text"
          aria-label="Fault log process"
          placeholder="Process"
          value={filterState.process}
          onChange={(event) => updateFilter({ process: event.target.value })}
        />
        <input
          type="text"
          inputMode="numeric"
          aria-label="Fault log PID"
          placeholder="PID"
          value={filterState.pid}
          onChange={(event) => updateFilter({ pid: event.target.value })}
        />
        <label><input type="checkbox" checked={filterState.regex} onChange={(event) => updateFilter({ regex: event.target.checked })} />Regex</label>
        <label><input type="checkbox" checked={filterState.matchCase} onChange={(event) => updateFilter({ matchCase: event.target.checked })} />Match Case</label>
        <button type="button" aria-label="Clear Fault Log Filters" onClick={() => setFilterState(EMPTY_DEVICE_FAULT_LOG_FILTER)}>Clear</button>
        {filter.error ? <span className="filter-error" role="alert">{filter.error}</span> : null}
      </div>
      {visibleEntries.length > 0 ? (
        <div className="fault-log-workspace">
          <ol className="fault-log-list" aria-label="Fault Log Entries">
            {visibleEntries.map((entry) => (
              <li key={entry.id}>
                <button
                  type="button"
                  className={entry.id === selected?.id ? "is-selected" : ""}
                  aria-label={`${entry.reason}: ${entry.summary}`}
                  onClick={() => setSelectedId(entry.id)}
                >
                  <strong>{entry.reason}</strong>
                  <span>{entry.summary}</span>
                  <small>{entry.processName} · PID {entry.pid}</small>
                </button>
              </li>
            ))}
          </ol>
          <section className="fault-log-inspector" aria-label="Fault Log Inspector">
            <div className="fault-log-metadata">
              <div>
                <strong>{selected?.summary}</strong>
                <span>{selected?.timestamp}</span>
              </div>
              <div className="fault-log-copy-actions">
                <button type="button" aria-label="Copy Fault Summary" onClick={() => void copy(selected?.copySummary ?? "", "summary")}>Copy summary</button>
                <button type="button" aria-label="Copy Fault Raw" onClick={() => void copy(selected?.raw ?? "", "raw log")}>Copy raw</button>
              </div>
            </div>
            <pre>{selected?.raw.split("\n").map((line, index) => <span key={`${index}:${line}`}>{line}{"\n"}</span>)}</pre>
          </section>
        </div>
      ) : (
        <div className="empty-state">
          <span className="empty-glyph" aria-hidden="true">⚠</span>
          <p>{loading
            ? "Refreshing fault logs…"
            : entries.length > 0
              ? "No fault log entries match the current filters."
              : result?.message ?? "Refresh fault logs to inspect device faults."}</p>
          <small>{result?.command || "No device data has been requested."}</small>
        </div>
      )}
      <footer className="panel-footer">
        <span>{result?.status.toUpperCase() ?? "IDLE"}</span>
        <span>{visibleEntries.length.toLocaleString()} / {entries.length.toLocaleString()} FAULTS</span>
        <span>{deviceId || "NO TARGET"}</span>
      </footer>
    </div>
  );
}

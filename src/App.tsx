import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import type { ArkLogApi, DeviceLogDevice, DeviceLogOutputBatch } from "./arklog-api";
import { DeviceFaultLogPanel } from "./DeviceFaultLogPanel";
import { DeviceLogFindBar } from "./DeviceLogFindBar";
import { DeviceLogHighlightedText } from "./DeviceLogHighlightedText";
import type { DeviceLogLevel } from "./device-log-entry";
import { DeviceLogSession } from "./device-log-session";
import { DeviceLogVirtualWindow } from "./device-log-virtual-window";
import { DeviceLogFilterBar } from "./DeviceLogFilterBar";
import {
  DeviceLogFilter,
  EMPTY_DEVICE_LOG_FILTER,
  type DeviceLogFilterState,
} from "./device-log-filter";

type AppProps = {
  api: ArkLogApi;
};

const LOG_VIRTUAL_WINDOW = new DeviceLogVirtualWindow();

export function App({ api }: AppProps) {
  const [devices, setDevices] = useState<DeviceLogDevice[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState("");
  const [streamId, setStreamId] = useState<string | null>(null);
  const [startPending, setStartPending] = useState(false);
  const [stopPending, setStopPending] = useState(false);
  const [status, setStatus] = useState("Discovering devices…");
  const [outputReady, setOutputReady] = useState(false);
  const [logSession] = useState(() => new DeviceLogSession());
  const sessionSnapshot = useSyncExternalStore(logSession.subscribe, logSession.getSnapshot);
  const [activeTab, setActiveTab] = useState<"hiLog" | "faultLog">("hiLog");
  const [filterState, setFilterState] = useState(EMPTY_DEVICE_LOG_FILTER);
  const [followingLatest, setFollowingLatest] = useState(true);
  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [currentFindOrdinal, setCurrentFindOrdinal] = useState(0);
  const [findFocusRequest, setFindFocusRequest] = useState(0);
  const [viewportGeometry, setViewportGeometry] = useState({ scrollTop: 0, height: 0 });
  const activeStreamRef = useRef<string | null>(null);
  const startingDeviceRef = useRef<string | null>(null);
  const stagedStartBatchesRef = useRef<DeviceLogOutputBatch[]>([]);
  const stoppingStreamRef = useRef(false);
  const logViewportRef = useRef<HTMLDivElement | null>(null);
  const currentFindLineRef = useRef<HTMLLIElement | null>(null);

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

  useEffect(() => () => logSession.dispose(), [logSession]);

  useEffect(() => {
    function handleFindShortcut(event: KeyboardEvent) {
      if (findOpen && event.key === "Escape") {
        event.preventDefault();
        setFindOpen(false);
        return;
      }
      if (activeTab !== "hiLog" || event.key.toLocaleLowerCase() !== "f") return;
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      setFindOpen(true);
      setFindFocusRequest((current) => current + 1);
    }

    window.addEventListener("keydown", handleFindShortcut);
    return () => window.removeEventListener("keydown", handleFindShortcut);
  }, [activeTab, findOpen]);

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | null = null;
    setOutputReady(false);
    void api.subscribeToOutput((batch) => {
      if (batch.streamId === activeStreamRef.current) {
        logSession.append(batch.lines);
      } else if (batch.deviceId === startingDeviceRef.current) {
        stagedStartBatchesRef.current.push(batch);
      }
    }).then((nextUnsubscribe) => {
      if (active) {
        unsubscribe = nextUnsubscribe;
        setOutputReady(true);
      } else {
        nextUnsubscribe();
      }
    }).catch((error: unknown) => {
      if (active) setStatus(error instanceof Error ? error.message : String(error));
    });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [api, logSession]);

  const selectedDevice = devices.find((device) => device.id === selectedDeviceId) ?? null;
  const isStreaming = streamId !== null;
  const filter = new DeviceLogFilter(filterState);
  const logFind = logSession.find(findOpen ? findQuery : "");
  const normalizedFindOrdinal = logFind.normalize(currentFindOrdinal);
  const currentFindOccurrence = findOpen
    ? logFind.occurrences[normalizedFindOrdinal] ?? null
    : null;
  const virtualRange = LOG_VIRTUAL_WINDOW.range({
    totalRows: sessionSnapshot.visibleCount,
    scrollTop: viewportGeometry.scrollTop,
    viewportHeight: viewportGeometry.height,
    followingLatest,
    anchorIndex: currentFindOccurrence?.lineIndex,
  });
  const renderedEntries = logSession.visibleEntries(virtualRange.start, virtualRange.end);

  useLayoutEffect(() => {
    const viewport = logViewportRef.current;
    if (followingLatest && viewport) viewport.scrollTop = viewport.scrollHeight;
  }, [followingLatest, sessionSnapshot.visibleCount]);

  useLayoutEffect(() => {
    if (findOpen && currentFindOccurrence) {
      currentFindLineRef.current?.scrollIntoView?.({ block: "nearest" });
    }
  }, [findOpen, findQuery, currentFindOrdinal, currentFindOccurrence?.lineIndex]);

  function updateFilter(patch: Partial<DeviceLogFilterState>) {
    const next = { ...filterState, ...patch };
    setFilterState(next);
    logSession.setFilter(next);
  }

  function handleLogScroll() {
    const viewport = logViewportRef.current;
    if (!viewport) return;
    setViewportGeometry({ scrollTop: viewport.scrollTop, height: viewport.clientHeight });
    const distanceFromLatest = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight;
    setFollowingLatest(distanceFromLatest <= 8);
  }

  function returnToLatest() {
    const viewport = logViewportRef.current;
    if (viewport) viewport.scrollTop = viewport.scrollHeight;
    setFollowingLatest(true);
  }

  function moveFind(direction: 1 | -1) {
    setCurrentFindOrdinal((current) => logFind.move(current, direction));
    if (logFind.count > 0) setFollowingLatest(false);
  }

  async function startStream() {
    if (!selectedDeviceId || !outputReady || startingDeviceRef.current) return;
    startingDeviceRef.current = selectedDeviceId;
    stagedStartBatchesRef.current = [];
    setStartPending(true);
    setStatus("Starting stream…");
    try {
      const stream = await api.startStream(selectedDeviceId);
      const stagedBatches = stagedStartBatchesRef.current;
      stagedStartBatchesRef.current = [];
      startingDeviceRef.current = null;
      activeStreamRef.current = stream.streamId;
      setStreamId(stream.streamId);
      logSession.clear();
      stagedBatches.forEach((batch) => {
        if (batch.streamId === stream.streamId) logSession.append(batch.lines);
      });
      setFollowingLatest(true);
      setStatus(`Streaming ${stream.deviceId}`);
    } catch (error) {
      startingDeviceRef.current = null;
      stagedStartBatchesRef.current = [];
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setStartPending(false);
    }
  }

  async function stopStream() {
    if (!streamId || stoppingStreamRef.current) return;
    stoppingStreamRef.current = true;
    setStopPending(true);
    setStatus("Stopping…");
    try {
      await api.stopStream(streamId);
      activeStreamRef.current = null;
      setStreamId(null);
      setStatus("Stopped");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      stoppingStreamRef.current = false;
      setStopPending(false);
    }
  }

  return (
    <main className="workbench">
      <section className="control-strip" aria-label="Stream controls" style={{ height: 48 }}>
        <div className={`device-field ${devices.length > 0 ? "has-device" : "is-empty"}`}>
          <span className="field-label">TARGET DEVICE</span>
          {devices.length > 0 ? (
            <select
              aria-label="Device"
              value={selectedDeviceId}
              disabled={startPending || streamId !== null}
              onChange={(event) => setSelectedDeviceId(event.target.value)}
            >
              {devices.map((device) => (
                <option key={device.id} value={device.id}>
                  {device.label} · {device.status}
                </option>
              ))}
            </select>
          ) : null}
          <div className={`runtime-status ${isStreaming ? "is-live" : ""}`}>
            <span className="status-dot" aria-hidden="true" />
            <span>{status}</span>
          </div>
        </div>
        <div className="workbench-tabs" role="tablist" aria-label="Log views">
          <button
            id="hi-log-tab"
            type="button"
            role="tab"
            aria-selected={activeTab === "hiLog"}
            aria-controls="log-workspace"
            onClick={() => setActiveTab("hiLog")}
          >
            HiLog
          </button>
          <button
            id="fault-log-tab"
            type="button"
            role="tab"
            aria-selected={activeTab === "faultLog"}
            aria-controls="log-workspace"
            onClick={() => setActiveTab("faultLog")}
          >
            Fault Log
          </button>
          {activeTab === "hiLog" ? (
            <div className="workbench-tab-tools">
              <div className="line-meter">
                <strong>{sessionSnapshot.totalCount.toLocaleString()}</strong>
                <span>lines buffered</span>
              </div>
            </div>
          ) : null}
        </div>
        <div className="stream-actions">
          {streamId ? (
            <button
              className="stream-button stop"
              type="button"
              aria-label="Stop stream"
              disabled={stopPending}
              onClick={() => void stopStream()}
            >
              <span aria-hidden="true">■</span> Stop stream
            </button>
          ) : (
            <button
              className="stream-button start"
              type="button"
              aria-label="Start stream"
              disabled={startPending || !outputReady || !selectedDevice || selectedDevice.status !== "online"}
              onClick={() => void startStream()}
            >
              <span aria-hidden="true">▶</span> Start stream
            </button>
          )}
        </div>
      </section>
      <div className="workbench-content">
        <section
          id="log-workspace"
          className="log-panel"
          role="tabpanel"
          aria-labelledby={activeTab === "hiLog" ? "hi-log-tab" : "fault-log-tab"}
        >
          <div className="log-panel-view hi-log-view" hidden={activeTab !== "hiLog"}>
            <DeviceLogFilterBar
              state={filterState}
              error={filter.error}
              onChange={updateFilter}
              onClearLogs={() => {
                logSession.clear();
                setFollowingLatest(true);
              }}
            />
            <div className="log-viewport-shell">
              {findOpen ? (
                <DeviceLogFindBar
                  query={findQuery}
                  current={logFind.count === 0 ? 0 : normalizedFindOrdinal + 1}
                  total={logFind.count}
                  focusRequest={findFocusRequest}
                  onChange={(query) => {
                    setFindQuery(query);
                    setCurrentFindOrdinal(0);
                    if (query.length > 0) setFollowingLatest(false);
                  }}
                  onMove={moveFind}
                  onClose={() => setFindOpen(false)}
                />
              ) : null}
              <div
                ref={logViewportRef}
                className="log-viewport"
                role="region"
                aria-label="HiLog output"
                onScroll={handleLogScroll}
              >
                {sessionSnapshot.visibleCount === 0 ? (
                  <div className="empty-state">
                    <span className="empty-glyph" aria-hidden="true">⌁</span>
                    <p>Waiting for log output.</p>
                    <small>Select an online device and start the stream.</small>
                  </div>
                ) : (
                  <ol className="log-lines" style={{ height: virtualRange.totalHeight }}>
                    {renderedEntries.map((entry, localIndex) => {
                      const index = virtualRange.start + localIndex;
                      const isCurrentLine = currentFindOccurrence?.lineIndex === index;
                      return (
                        <li
                          key={`${index}:${entry.raw}`}
                          ref={isCurrentLine ? currentFindLineRef : undefined}
                          className={severityClass(entry.level)}
                          aria-current={isCurrentLine ? "true" : undefined}
                          data-line-number={index + 1}
                          style={{
                            top: index * virtualRange.rowHeight,
                            height: virtualRange.rowHeight,
                          }}
                        >
                          <code>
                            <DeviceLogHighlightedText
                              raw={entry.raw}
                              decorations={[
                                ...filter.matchRanges(entry).map((range) => ({
                                  ...range,
                                  className: "regex-match",
                                })),
                                ...(findOpen ? logFind.occurrencesForLine(index).map((occurrence) => ({
                                  start: occurrence.start,
                                  end: occurrence.end,
                                  className: occurrence.ordinal === normalizedFindOrdinal
                                    ? "find-match is-current"
                                    : "find-match",
                                })) : []),
                              ]}
                            />
                          </code>
                        </li>
                      );
                    })}
                  </ol>
                )}
              </div>
              {!followingLatest && sessionSnapshot.visibleCount > 0 ? (
                <button
                  className="back-to-latest"
                  type="button"
                  aria-label="Back to latest"
                  onClick={returnToLatest}
                >
                  Back to latest ↓
                </button>
              ) : null}
            </div>
            <footer className="panel-footer">
              <span>
                {sessionSnapshot.visibleCount.toLocaleString()} / {sessionSnapshot.totalCount.toLocaleString()} VISIBLE
              </span>
              <span>{selectedDevice?.id ?? "NO TARGET"}</span>
            </footer>
          </div>
          <DeviceFaultLogPanel
            api={api}
            deviceId={selectedDeviceId}
            hidden={activeTab !== "faultLog"}
            onStatusChange={setStatus}
          />
        </section>
      </div>
    </main>
  );
}

function severityClass(level: DeviceLogLevel) {
  if (level === "error" || level === "fatal") return "severity-error";
  if (level === "warn") return "severity-warning";
  return "";
}

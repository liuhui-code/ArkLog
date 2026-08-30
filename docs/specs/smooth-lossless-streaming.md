# Smooth lossless live streaming specification

Status: Accepted.

## Goal

Keep ArkLog responsive and visually stable during sustained HiLog traffic while
preserving every line delivered for the active in-memory session.

## Public behavior

1. ArkLog applies no frontend line cap. Every non-empty batch line delivered
   for the active stream remains in the current session until `Clear logs`, a
   new stream, or application shutdown.
2. Batches that arrive after start is requested but before its `streamId` is
   returned are staged and applied in original order once that stream is
   confirmed. Batches for any other stream are ignored.
3. Clearing is generation-safe: all lines delivered before the clear action are
   removed, including lines whose repaint notification is pending; later lines
   remain eligible for display.
4. React receives at most one session notification per animation frame during
   a burst. Coalescing notifications never coalesces away source lines.
5. Filtering is incremental during steady streaming: an unchanged regular
   expression is evaluated only against newly appended lines. Changing the
   expression re-indexes the complete session once.
6. The log viewport renders a fixed-height window plus overscan instead of one
   DOM row per retained line. A stress view with 10,000 retained lines mounts at
   most 200 log rows.
7. While following latest, the rendered window and scroll position remain at
   the newest line without showing an intermediate empty view.
8. While scrolled away, incoming batches preserve `scrollTop` and the visible
   row anchor. `Back to latest` explicitly resumes following.
9. Find navigation can locate a match outside the mounted window and bring its
   line into view without rendering all intervening rows.
10. Stopping a Rust stream waits for its reader/aggregator worker to flush the
    final partial batch before the stop operation completes.
11. Start and stop commands are single-flight. Their controls are disabled
    while pending, and device selection cannot change during start.
12. An open find operation incrementally indexes appended visible lines;
    navigation and rendering do not rescan the complete session.

## Object boundaries

- `DeviceLogSession` owns raw entries, incremental filter/find indexes,
  immutable cached snapshots, burst notification scheduling, and clear
  generations. It is independent of React and Tauri.
- `DeviceLogVirtualWindow` maps retained-row counts and viewport geometry to a
  bounded render range.
- `App` subscribes through React's external-store interface, owns stream
  start/stop staging, and renders only the virtual range.
- `DeviceLogRuntime` owns the HDC child and its aggregation worker as one active
  stream lifecycle.

## Operational boundary

ArkLog introduces no intentional dropping or eviction. Like every in-memory
process, it cannot guarantee an infinite stream after operating-system memory
exhaustion. Tests cover 100,000 retained domain lines and a 10,000-line UI
burst without imposing a retention limit.

## Acceptance evidence

- A public UI stress test proves 10,000 lines are retained while DOM rows remain
  bounded and the latest line is present.
- Session tests prove ordered burst retention, immutable snapshot caching,
  incremental filtering, empty batches, clear generations, and 100,000 lines.
- Public UI tests prove subscription readiness, start-race staging,
  stale-stream rejection, single-flight lifecycle commands, stop-tail delivery,
  stable scroll-away behavior, and off-window find navigation.
- Rust tests prove high-volume ordered batching and final flush on stop.

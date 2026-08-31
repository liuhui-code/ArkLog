# Ratatui low-memory runtime specification

Status: Accepted.

Supersedes the React/Tauri host boundaries in the accepted live-log specs.
The log behavior in those specs remains authoritative.

## Goal

Run ArkLog as one Windows/macOS terminal process that can follow an unbounded
device log session without allowing retained log text to grow the process RSS
without bound.

## Public behavior

1. `arklog` opens one Ratatui workspace with a refreshable connected-device
   list, device state, HiLog/Fault Log tabs, regular-expression filtering,
   in-view find, scrolling, clear, and follow-latest controls.
2. The same Crossterm application runs in Windows Terminal/PowerShell and
   macOS Terminal/iTerm2. Terminal state is restored after normal exit or an
   application error.
3. HiLog starts for the selected online device and every complete source line
   remains available, in source order, until clear, stream replacement, or
   process exit.
4. HiLog text and line indexes live in a process-lifetime temporary session
   store. The store is removed on clear or exit and is never reopened on a
   later ArkLog start. ArkLog has no SQLite or durable history.
5. Memory contains only bounded stream batches, compiled queries, view state,
   Fault Log response data, and the currently rendered HiLog window. Scrolling
   reads the requested window from the session store.
6. Producer/consumer queues are bounded. Saturation applies backpressure to
   HDC instead of dropping source lines or growing memory.
7. A regular expression has explicit pattern and compiled-size limits. Invalid
   or over-budget expressions report an inline error and never mutate raw
   session data.
8. Changing the expression re-indexes the complete current session. Appended
   lines are evaluated incrementally while the expression is unchanged.
9. Find is case-insensitive literal search over regex-visible HiLog lines.
   Navigation wraps, pauses follow-latest, and reveals the current result.
10. The release process must remain below 50 MiB RSS during the automated
    100,000-line stress scenario on each supported desktop platform.
11. Character commands require the Control modifier. Device refresh remains
    available while HiLog is active; a refresh preserves an online selected
    stream and stops a stale stream when its device disappears or goes offline.
12. Device discovery, Fault Log fetch, and stream stop run as single-flight
    background jobs. HDC commands have deadlines and total-output limits, and
    connection, stream, and Fault Log states cannot overwrite each other.
13. Device and Fault Log lists plus raw diagnostics render only their visible
    terminal windows. Fault Log raw output is capped at 4 MiB.
14. Verbose HDC discovery is interpreted by its documented connection-state
    field. Existing device rows never hide a failed or stale refresh state.
15. Background results carry device identity. A result for a previously
    selected device is discarded, and an HDC process exit immediately replaces
    `LIVE` with its real inactive error state after committing its final batch.
16. The terminal redraws only after state, input, terminal size, or visible log
    data changes. Each event-loop tick consumes at most eight log batches.
17. A filter change scans raw records once while rebuilding filter and find
    indexes together. A find-only change evaluates visible records without
    re-running the regex. Unchanged queries perform no rebuild.

## Object boundaries

- `arklog-core` owns HDC discovery, command construction, lossless bounded
  batching, and stream lifecycle. It has no Ratatui or Tauri dependency.
- `SessionLogStore` owns temporary raw/index files, regex validation,
  incremental indexing, bounded window reads, and clear-on-drop cleanup.
- `ArkLogController` owns device selection, HDC jobs, stream truth, and bounded
  batch delivery. `TerminalApp` owns tabs, input modes, scrolling, and redraw
  scheduling.
- `ui` renders immutable view data with Ratatui and performs no HDC or file I/O.

## Memory budget

- HiLog delivery channel: at most 8 batches of 50 lines.
- HDC reader-to-batcher queue: at most 256 complete lines; a slow consumer
  backpressures the reader instead of accumulating an unbounded burst.
- Rendered HiLog window: terminal height plus at most 32 overscan lines.
- HDC discovery output: at most 256 KiB; Fault Log output: at most 4 MiB.
- Regex source: at most 4 KiB; compiled regex budget: at most 1 MiB.
- Application RSS acceptance ceiling: 50 MiB in release mode.
- Raw HiLog and line/match indexes: session temporary files, not heap buffers.

## Non-goals

- retaining HiLog after ArkLog exits
- SQLite, history queries, retention configuration, or background databases
- graphical window chrome, mouse-only interaction, or browser rendering
- promising finite disk use for an intentionally unbounded no-loss session

## Acceptance evidence

- Store tests append at least 100,000 lines, retrieve old/latest windows in
  order, re-index by regex, clear, and prove the retained heap estimate stays
  bounded independently of raw bytes.
- Stream tests prove queue backpressure and final partial-batch delivery do not
  drop or reorder lines, including asynchronous stop and unexpected exit.
- Controller tests prove idle work reports no redraw activity, per-tick log
  pumping is bounded, stale device results are rejected, and displayed stream
  state follows the real child process.
- TestBackend tests prove the shared tabs, status, filter/find, Fault Log, and
  follow-latest states render through the public UI interface.
- Real-HDC boundaries remain covered with fake command fixtures.
- macOS and Windows CI run the release stress scenario with 100,000 HiLog lines
  and a 2 MiB Fault Log payload, asserting RSS remains below 50 MiB.

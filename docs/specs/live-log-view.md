# Live log view specification

Status: Accepted.

## Goal

Keep ArkLog focused exclusively on watching a phone's live, unmodified HiLog
stream while the user operates that phone.

## Public behavior

1. ArkLog preserves every complete raw line received during the current view;
   it does not shorten a line or evict an older displayed line because a
   frontend line limit was reached.
2. The HiLog query is always interpreted as a case-insensitive regular
   expression. An empty expression matches every line, and an invalid
   expression reports an inline error without modifying any raw line.
3. Changing the regular expression re-evaluates the current raw-line buffer.
   After that change, only newly received lines matching the active expression
   become visible.
4. Receiving a new batch never removes a line that is already visible. Visible
   lines leave the view only when the user changes the expression or activates
   `Clear logs`.
5. `Clear logs` clears the current in-memory HiLog view without stopping an
   active stream. Later matching lines continue to appear.
6. The log viewport owns vertical and horizontal scrolling. While the user is
   at the end, incoming visible lines keep the viewport at the latest line.
7. Scrolling away from the end pauses automatic following and exposes a
   `Back to latest` action. Activating it returns to the end and resumes
   following.
8. ArkLog does not persist HiLog output. Starting a stream begins an empty
   in-memory session, and closing or restarting ArkLog discards that session.
9. The workbench exposes no history loading, storage health, retention, or
   persisted-history clearing controls.
10. Every non-empty substring matched by the active regular-expression filter
    is highlighted without changing the raw line. In-view find highlighting is
    a separate decoration and can overlap it.

## Object boundaries

- `DeviceLogFilter` owns regular-expression compilation, validation, and raw
  line matching. It never truncates its input.
- `App` owns the unmodified session buffer, expression changes, visible-view
  clearing, and follow-latest state.
- `ArkLogApi` exposes only live stream operations for HiLog. The Tauri event
  sink forwards each batch directly to React without a storage intermediary.
- `arklog-core` owns device discovery and stream lifecycle but no database.

## Non-goals

- mutating a raw line while presenting regular-expression filter highlights
- SQLite, persisted HiLog history, retention, or storage health
- a frontend retention limit
- changing Fault Log filtering

## Acceptance evidence

- A filter test proves a matching line longer than 4,096 characters is kept
  intact.
- Public UI tests prove expression changes re-evaluate existing lines, new
  batches preserve visible lines, and `Clear logs` clears only the view.
- A public UI test proves scroll-away, continued receipt, and `Back to latest`.
- A public UI test proves history and storage controls are absent.
- Boundary and dependency checks prove no history commands or SQLite dependency
  remain.

# SQLite log history specification

Status: Superseded by `live-log-view.md`; ArkLog no longer persists HiLog.

Source behavior: ArkLine Device Log at revision
`e8e91334ecc71db172be01d623c79cafb731987d`.

## Goal

Keep delivered HiLog batches across application restarts and let the user load
recent logs for the selected device without changing the live-stream contract.

## Public behavior

1. Every delivered live batch is appended to an application-data SQLite
   database before the host forwards the batch event to React.
2. Stored rows retain stream ID, device ID, receive time, and raw log text.
3. A history query requires a device ID and returns at most 500 of that
   device's newest rows, ordered oldest-to-newest for display.
4. `Load history` is available while no live stream is running. It replaces the
   current rendered buffer with the selected device's recent stored rows.
5. The current HiLog regular expression applies to loaded history exactly as it
   applies to live rows.
6. Starting a new live stream replaces the loaded snapshot with new live data,
   preserving the existing stream behavior.
7. A history-query error is reported through the existing status readout.
8. A persistence failure must not suppress delivery of the live batch.

## Object boundaries

- `DeviceLogStore` owns schema initialization, transactional batch writes, and
  bounded recent-history queries. It remains in `arklog-core` and has no Tauri
  dependency.
- The Tauri event sink composes durable storage with host event delivery.
- `ArkLogApi` owns the frontend history-query boundary.
- `App` owns whether the current view is a historical snapshot or live output;
  it does not know SQLite details.

## Storage schema

`device_log_lines` uses an integer sequence key and stores `stream_id`,
`device_id`, `received_at_ms`, and `raw`. An index on device and descending
sequence supports the bounded recent query. Schema creation is idempotent.

## Non-goals

- server-side text or structured filtering
- cursor pagination, cancellation, scan budgets, or query deadlines
- retention, storage health, export, or clearing history
- segmented log files and metadata pruning
- migration of alternate vendor-specific HiLog layouts

## Acceptance evidence

- A core integration test stores multiple batches/devices and proves bounded,
  chronological recent history across a database reopen.
- A frontend boundary test proves the Tauri command mapping.
- A public UI test proves that loaded history is displayed and filtered.

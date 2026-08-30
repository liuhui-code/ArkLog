# Device log storage governance specification

Status: Superseded by `live-log-view.md`; ArkLog has no HiLog storage.

Source behavior: ArkLine Device Log at revision
`e8e91334ecc71db172be01d623c79cafb731987d`.

## Goal

Keep durable device logs bounded, expose understandable storage health, and let
the user clear ArkLog history without risking unrelated application data.

## Public behavior

1. Durable live writes retain the newest 100,000 rows globally and remove only
   older `device_log_lines` rows after a successful batch insert.
2. Retention preserves sequence order and never suppresses downstream live
   event delivery.
3. Storage health reports database bytes, stored line count, configured line
   limit, oldest/newest receive times, pressure state, and recommended action.
4. Pressure is `healthy` below 80% of the line limit, `warning` from 80% up to
   the limit, and `critical` at the limit.
5. Recommended actions are `none`, `reviewRetention`, and `clearOldLogs` for
   healthy, warning, and critical pressure respectively.
6. `Clear history` is unavailable while a live stream or another storage
   action is active.
7. The first clear action only reveals `Confirm clear history`; only the second
   explicit action calls the backend. The user can cancel confirmation.
8. A confirmed clear transaction deletes only rows from `device_log_lines`,
   keeps the SQLite database/schema intact, clears the historical view, and
   refreshes health.
9. Errors use the existing status readout and leave confirmation available for
   an explicit retry or cancellation.

## Object boundaries

- `DeviceLogStore` owns retention, aggregate health queries, pressure
  classification, and transactional table clearing.
- `PersistingLogBatchSink` composes write, retention, and downstream delivery;
  it does not expose SQLite details.
- Tauri commands expose health and clear results from the managed store.
- `ArkLogApi` owns the frontend host boundary.
- `DeviceLogStorageControls` owns presentation and two-step confirmation; it
  emits actions but does not mutate storage itself.

## Data contracts

`DeviceLogStorageHealth` contains `databaseBytes`, `lineCount`, `maxLineCount`,
`oldestReceivedAtMs`, `newestReceivedAtMs`, `pressureState`, and
`recommendedAction`.

`DeviceLogStorageClearResult` contains `removedLineCount`.

## Non-goals

- byte-target retention planning or segment-file deletion
- configurable user retention preferences
- deleting the database file or resetting SQLite sequence values
- VACUUM, WAL checkpoint controls, or background maintenance scheduling
- per-device quotas

## Acceptance evidence

- A core integration test proves automatic retention keeps only the newest
  configured rows.
- Core tests prove health classification and transactional clear behavior.
- Frontend boundary tests prove health/clear command mapping.
- A public UI test proves no backend clear occurs before the second explicit
  confirmation and that successful clear resets the historical view.

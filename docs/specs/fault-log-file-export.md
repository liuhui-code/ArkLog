# Fault Log file export specification

Status: Accepted as the next migration slice.

Depends on `docs/specs/fault-log-mvp.md` and
`docs/specs/fault-log-filter-copy.md`.

## Goal

Let developers save the currently visible fault diagnostics as a portable text
file without exposing filesystem or dialog APIs to React.

## Public behavior

1. `Export Visible Fault Logs` is enabled only when at least one filtered entry
   is visible and no export is active.
2. Export preserves the current visible order and includes only visible entries,
   not entries hidden by filters.
3. The text format starts with an ArkLog header containing the device id and
   entry count. Each entry follows under a separator containing its id, with the
   raw device payload unchanged inside that section.
4. Export opens the native Tauri save dialog with a sanitized default filename
   `arklog-<device>-faults.txt` and a text-file filter.
5. Confirming the dialog creates or replaces only the chosen file. It does not
   modify device logs or the in-memory result.
6. Cancelling the dialog returns a normal cancelled result and does not report
   an error.
7. Success reports the exported entry count and chosen path through the existing
   status readout. Dialog or write failures are also reported there.

## Data contracts

`DeviceFaultLogExportResult` contains `path`, `entryCount`, and `bytesWritten`.

`ArkLogApi.exportFaultLogs(deviceId, entries)` returns that result, or `null`
when the user cancels the native dialog.

## Object boundaries

- `DeviceFaultLogExporter` owns deterministic text formatting and file writing
  in `arklog-core`; it has no Tauri dependency.
- The Tauri command owns the native save dialog and delegates writing to the
  core exporter.
- `ArkLogApi` remains the only export boundary visible to React.
- `DeviceFaultLogPanel` supplies its current visible raw entries and owns only
  busy/status presentation.

## Non-goals

- JSON, ZIP, HTML, or diagnostic-bundle export
- exporting hidden entries or HiLog output
- automatic naming without user confirmation
- directory export, append mode, sharing, or upload
- device-side deletion or mutation

## Acceptance evidence

- A core public-interface test proves deterministic content, visible ordering,
  raw preservation, byte count, and replacement of the selected file.
- A frontend boundary test proves selected device and entries map to the Tauri
  command and cancellation remains `null`.
- A public UI test proves only visible entries are exported and success is
  reported without clearing the view.

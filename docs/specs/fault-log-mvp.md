# Fault Log MVP specification

Status: Accepted as the next migration slice.

Source behavior: ArkLine Fault Log at revision
`e8e91334ecc71db172be01d623c79cafb731987d`.

## Goal

Let a developer refresh and inspect device fault diagnostics without searching
the live HiLog stream.

## Public behavior

1. The workbench exposes internal `HiLog` and `Fault Log` tabs. Existing HiLog
   state remains intact when switching tabs.
2. `Refresh Fault Logs` requests diagnostics for the selected device and is
   disabled when no device is selected or a refresh is active.
3. The HDC command uses the official Hiview dump surface:
   `hdc -t <device> shell hidumper -s 1201 -a "-p Faultlogger -l -d"`.
   Its execution remains behind the core `CommandRunner` boundary.
4. Current hidumper output is split at its `******` record delimiters. Legacy
   raw output remains supported by splitting at blank lines followed by a
   recognized field. Internal blank lines remain part of the entry.
5. Fetch results use structured states: `ready`, `empty`, `unavailable`,
   `unauthorized`, or `error`. Command failure text is returned as data rather
   than causing an unhandled UI exception.
6. The first returned entry is selected. Selecting another row updates the raw
   inspector.
7. Empty and failure states remain visible in the Fault Log panel and include a
   concise command-level message. A successful command that reports `dump
   operation is not permitted` is classified as unauthorized; `Service is not
   ready` is unavailable. Neither message is rendered as a ready log entry.
8. Changing the selected device clears the current Fault Log view so entries
   from different devices are never mixed.

## Data contracts

`DeviceFaultLogFetchResult` contains `deviceId`, `entries`, `command`, `stderr`,
`status`, and `message`.

Each `DeviceFaultLogRawEntry` contains a stable fetch-local `id` and its exact
`raw` diagnostic text.

## Object boundaries

- `HdcClient` owns construction and execution of HDC device commands.
- The core Fault Log domain owns output normalization, entry splitting, and
  structured failure classification; it has no Tauri dependency.
- Tauri exposes one thin `list_device_fault_logs` command.
- `ArkLogApi` remains the only host boundary visible to React.
- `DeviceFaultLogPanel` owns Fault Log presentation and selection while `App`
  owns shared device and tab state.

## Non-goals

- deleting fault logs from the device
- persistence or export of fault entries
- type, PID, process, text, or regular-expression filtering
- clipboard actions and stack symbolication
- fallback probing of undocumented device filesystem paths
- automatic refresh or multi-device comparison

## Acceptance evidence

- A core public-interface test proves command construction, entry splitting,
  and internal blank-line preservation.
- Core tests prove empty, unavailable, unauthorized, and generic error states.
- A frontend boundary test proves the Tauri command and device argument.
- A public UI test proves tab switching, refresh, first-entry selection, row
  selection, and raw inspection.

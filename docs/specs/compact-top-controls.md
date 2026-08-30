# Compact top controls specification

Status: Accepted for the current migration slice.

## Goal

Represent device selection, connection, and runtime feedback as one device
state instead of repeating the same meaning across separate controls.

## Public behavior

1. The workbench has no standalone application banner above device controls.
2. Device selection and runtime status share one device-state cell; there is no
   independent `CONNECTION` readout.
3. On the desktop layout the control region is 48 CSS pixels high, and its
   device, connection/status, tab, and action groups remain single-line.
4. Runtime status remains visible and continues to show discovery, streaming,
   fault-log, export, and error messages.
5. The shared HiLog/Fault Log workspace moves upward and keeps all existing
   behavior.
6. Narrow layouts may hide secondary connection detail, but retain device
   selection, tabs, and stream actions.
7. No visible `AL` or `ArkLog` brand block consumes space in the control region.
8. While discovery is empty, the device-state cell shows exactly one
   `No devices` message and does not also show `unavailable`.

## Object boundaries

- `App` continues to own status and device-control presentation.
- `App` depends on the live-only API and does not expose history actions.

## Non-goals

- redesigning the log workspace or filters
- removing status feedback
- adding a title-bar or native window integration

## Acceptance evidence

- A public React test observes runtime status and the log tabs inside the
  48-pixel stream-control region, with no visible ArkLog heading or separate
  banner.
- An empty-device test observes one `No devices` state and no duplicate
  connection-unavailable state.
- Existing workspace and behavior tests remain green.

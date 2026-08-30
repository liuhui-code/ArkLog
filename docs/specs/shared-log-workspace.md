# Shared log workspace specification

Status: Accepted for the current migration slice.

Depends on `docs/specs/fault-log-mvp.md`.

## Goal

Present HiLog and Fault Log as two views of one stable work area instead of two
separate panels.

## Public behavior

1. The workbench exposes one log workspace below the device controls.
2. `HiLog` and `Fault Log` remain mutually exclusive tabs controlling that same
   workspace, embedded in the compact device/status control strip.
3. The selected tab changes only the content inside the workspace; it does not
   add a second bordered panel or move the workspace.
4. HiLog is selected initially. Selecting Fault Log shows its refresh, filter,
   inspector, copy, and export controls in the same area.
5. HiLog has no redundant `LIVE CHANNEL / HiLog output` heading row. Its filter
   controls start at the top of the shared workspace; the live buffer count
   remains in the embedded tabs.
6. Switching tabs preserves each view's current in-memory state and filters.
7. Each tab exposes `aria-selected`, and both identify the shared workspace they
   control.

## Object boundaries

- `App` owns the shared workspace shell and active-tab state.
- `DeviceFaultLogPanel` owns only Fault Log content and behavior; it no longer
  creates an independent outer panel.
- The workspace depends on the live-only `ArkLogApi` and has no history UI.

## Non-goals

- merging HiLog and Fault Log data or filters
- changing streaming, fault refresh, copy, or export behavior
- loading Fault Log automatically when its tab is selected
- adding additional tabs or detachable windows

## Acceptance evidence

- A public React test observes exactly one stable tab panel before and after
  switching, with the selected view rendered inside it.
- Existing HiLog and Fault Log behavior tests remain green.

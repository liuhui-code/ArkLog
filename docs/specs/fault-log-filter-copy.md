# Fault Log filtering and copy specification

Status: Accepted as the next migration slice.

Depends on `docs/specs/fault-log-mvp.md`.

## Goal

Let developers narrow a fetched Fault Log result to a relevant diagnostic and
copy either a concise structured summary or the exact device payload.

## Public behavior

1. Raw entries are classified as `jsCrash`, `cppCrash`, `appFreeze`,
   `appKilled`, `sysWarning`, or `unknown` from conservative `Reason` values.
   Unknown content remains inspectable.
2. Fault Log filters combine type, exact numeric PID, case-aware process text,
   and a text query using AND semantics.
3. Text query defaults to escaped plain text. Regex mode uses the entered
   regular expression and `Match Case` controls both query and process matching.
4. Invalid PID or regex input is reported inline and matches no rows without
   removing fetched entries.
5. Regex evaluation is skipped for raw entries longer than 4,096 characters.
6. When filtering hides the selected entry, the first visible entry becomes the
   inspected and copied entry. Clearing filters restores the fetched result.
7. `Copy Fault Summary` writes a stable multiline summary containing reason,
   summary, process, and PID. `Copy Fault Raw` writes the exact raw payload.
8. Clipboard access goes through `ArkLogApi`; React does not call a host API
   directly. Success and failure use the existing status readout.

## Object boundaries

- `DeviceFaultLogEntry` owns field parsing, conservative type classification,
  and summary formatting.
- `DeviceFaultLogFilter` owns validation and matching; it does not mutate
  entries or UI state.
- `DeviceFaultLogPanel` owns controlled filter state, visible selection, and
  user actions.
- `ArkLogApi.writeClipboard` owns the browser/Tauri host clipboard boundary.

## Non-goals

- fuzzy search, saved presets, highlighting, or result ranking
- exporting files or diagnostic bundles
- copying stack frames individually
- device-side deletion or mutation
- broad keyword inference that classifies ordinary prose as a fault type

## Acceptance evidence

- Domain tests prove conservative classification, combined matching, plain
  metacharacters, case-sensitive regex, validation, and long-input protection.
- A boundary test proves clipboard text is forwarded through `ArkLogApi`.
- Public UI tests prove combined filtering, inline regex errors, visible
  selection fallback, summary copy, and exact raw copy.

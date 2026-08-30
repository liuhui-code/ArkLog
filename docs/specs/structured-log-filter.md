# Structured log filter specification

Status: Superseded by `live-log-view.md` for HiLog. Fault Log keeps its own
filtering specification.

Source behavior: ArkLine Device Log at revision
`e8e91334ecc71db172be01d623c79cafb731987d`.

## Goal

Let a user narrow buffered HiLog output by parsed log metadata in addition to
the existing text and regular-expression query.

## Parsed HiLog fields

ArkLog recognizes the standard line shape containing timestamp, PID, TID,
level, domain/tag, process, and message. Recognized level letters map to
Verbose, Debug, Info, Warn, Error, and Fatal. Unrecognized lines remain visible
as raw messages with level `unknown`.

## Public behavior

1. Error, Warn, Info, Debug, and Fatal are toggle buttons. With no level
   selected, every level is allowed; selected levels are combined with OR.
2. PID accepts digits only and matches the parsed PID exactly.
3. Invalid PID text displays `PID filter must be a number` and matches no logs.
4. Process, Domain, and Tag use substring matching.
5. `Match Case` applies to Process, Domain, and Tag as well as the existing
   query.
6. Different filter categories combine with AND.
7. `Clear Filters` restores the complete buffered view without removing logs.
8. Lines that do not match the standard HiLog shape still participate in raw
   text and regex filtering, but have no structured field values.

## Object boundaries

- `DeviceLogEntry` owns parsing and the immutable representation of one line.
- `DeviceLogFilter` owns validation and all matching rules.
- `DeviceLogFilterBar` owns the controlled React inputs and emits immutable
  state patches; it does not filter data itself.

## Non-goals

- persistent storage or historical queries
- backend query pagination and cancellation
- parsing alternate vendor-specific HiLog layouts
- match highlighting

## Acceptance evidence

- Public UI tests prove level, PID, combined structured fields, validation, and
  clear behavior.
- Domain tests prove standard HiLog parsing and fallback behavior for raw lines.

# Regex log filter specification

Status: Superseded by `live-log-view.md`.

Source behavior: ArkLine Device Log at revision
`e8e91334ecc71db172be01d623c79cafb731987d`.

## Goal

Let a user narrow the currently buffered HiLog lines with plain text or a
regular expression without changing stream collection or evicting unmatched
lines.

## Public behavior

1. The log panel exposes a `Filter logs` text field.
2. Plain text mode is the default and matches case-insensitively.
3. Enabling `Regex` interprets the filter text as a JavaScript regular
   expression.
4. Enabling `Match Case` makes either mode case-sensitive.
5. An empty query displays every buffered line.
6. An invalid regular expression displays an inline validation error and no
   lines are treated as matches.
7. Clearing or changing the filter never removes lines from the 2,000-line
   in-memory buffer.
8. Regex matching skips individual log lines longer than 4,096 characters to
   keep arbitrary regular expressions out of the render path for large input.

## Design boundary

Filtering is encapsulated by an immutable `DeviceLogFilter` domain object.
React owns only the editable filter state and derives visible lines during
render. Stream collection and the `ArkLogApi` remain unchanged.

## Non-goals

- SQLite or historical log queries
- backend pagination and cancellation
- PID, process, domain, tag, or level filters
- match highlighting

## Acceptance evidence

- A public UI test proves default plain-text filtering.
- Public UI tests prove regex matching, case sensitivity, invalid-pattern
  feedback, and restoration of buffered lines after clearing the query.
- A domain test proves the long-line regex guard.

# Execution diagnostics specification

Status: Accepted.

## Goal

Give real users a small, shareable execution trace when ArkLog behaves
differently with a physical device, without persisting the HiLog or Fault Log
content that ArkLog exists to display.

## Public behavior

1. A normal ArkLog session writes `arklog-execution.log` in the operating
   system temporary directory. `ARKLOG_EXECUTION_LOG` overrides the path.
2. The trace contains application start/stop, accepted keyboard commands,
   connection/stream/Fault lifecycle changes, device count, and application or
   HDC errors. Every entry has a Unix-millisecond timestamp.
3. ArkLog never writes HiLog lines, Fault Log bodies, regex/find values, or
   clipboard content to this trace. Error messages can contain identifiers or
   paths returned by the operating system or HDC and should be reviewed before
   sharing publicly.
4. Unchanged runtime state creates no entry. Diagnostic events use a bounded
   128-entry queue and background writer, so diagnostics never block the live
   log path. A saturated diagnostic queue may discard diagnostic events; it
   never discards device log events.
5. The file is capped at 1 MiB. Before the next entry would exceed the cap,
   ArkLog truncates the old diagnostic segment and continues with the newest
   events.
6. Failure to create the trace reports `Execution log unavailable` in the
   existing status area but does not prevent ArkLog from running.
7. After restoring the terminal on exit, ArkLog prints the execution-log path
   so the user can attach it to an issue report.

## Object boundaries

- `ExecutionLog` owns structured formatting, bounded delivery, file writing,
  and the byte budget.
- `RuntimeDiagnostics` owns change detection for connection, stream, Fault Log,
  and device-count state. It does not own controller behavior.
- `TerminalApp` reports commands and observes controller state. The controller
  remains independent of diagnostic file I/O.

## Acceptance evidence

- Public tests prove structured events are flushed, file size remains bounded,
  duplicate state polls create no noise, and multiline errors remain one event
  per physical line.
- Existing lossless-stream and memory tests prove diagnostics do not change
  HiLog delivery, ordering, or the 50 MiB process budget.

# Stream intent control specification

Status: Accepted.

## Goal

Make every `Ctrl+S` press express a deterministic final user intention without
blocking the terminal, starting duplicate HDC streams, interrupting lossless
stop flushing, or presenting an intended state as if it were already real.

## Public behavior

1. ArkLog begins with one pending start intention while initial device
   discovery runs. The header shows `START QUEUED`.
2. Pressing `Ctrl+S` during that discovery cancels the pending start. A device
   becoming ready later does not override the cancellation.
3. Pressing `Ctrl+S` while stopped or inactive requests start when an online
   device is ready. Pressing it again before execution cancels that request.
4. Pressing `Ctrl+S` while live requests one asynchronous stop. The real state
   remains `STOPPING` until HDC termination and final-batch flushing finish.
5. Pressing `Ctrl+S` while stopping queues one restart and shows
   `RESTART QUEUED`. A second press during the same stop cancels that restart.
6. Pressing `Ctrl+S` while starting queues a stop that executes as soon as the
   stream becomes active.
7. Once a start or stop request is accepted, its pending intention is cleared;
   `StreamState` alone describes the real lifecycle. An unexpected HDC exit
   does not automatically restart a stream without a new pending intention.
8. If a stop fails while the old stream remains active, a queued restart is
   considered satisfied and cannot trigger a later surprise restart.
9. Every `Ctrl+S` command and its pending action are written to the bounded
   execution diagnostics trace. HiLog content is never written there.

## Object boundaries

- `StreamIntent` owns pending `Start`/`Stop` intention and reconciliation rules.
- `ArkLogController` remains the source of truth for connection and HDC stream
  lifecycle and executes at most one accepted action per reconciliation.
- `TerminalApp` forwards `Ctrl+S`, reconciles after background state changes,
  and renders real state plus a separate queued-intention hint.

## Acceptance evidence

- Pure state-machine tests cover refresh cancellation, starting cancellation,
  stopping restart, double-toggle cancellation, and active stop failure.
- Fake-HDC controller tests cover delayed discovery and exactly one replacement
  stream after the previous stream is fully reaped.
- TestBackend tests distinguish `STOPPING`, `STOPPED`, `START QUEUED`, and
  `RESTART QUEUED` without presenting a queued intention as actual state.

# Desired stream and runtime coordination specification

Status: Accepted.

## Goal

Make every `Ctrl+S` press express a persistent final user intention while one
Controller serializes discovery, HDC stream lifecycle, bounded-channel draining,
and restart decisions.

## Public behavior

1. `DesiredStream` has two durable values: `Running` and `Stopped`. Temporary
   device absence, discovery errors, start failures, and unexpected HDC exits do
   not erase `Running`.
2. User Stop is the only path that changes the desired value to `Stopped`.
   Device replacement and health cleanup stop an obsolete process without
   changing the desired value.
3. While `Running` has no usable device, Controller schedules one discovery job
   after approximately one second. Once the stream process is healthy, automatic
   device discovery is suspended so a second HDC command cannot interrupt or
   time out the active HiLog session. Manual refresh remains available.
4. The active HDC child process is the stream-health signal. An unexpected exit
   immediately makes discovery due; quiet output from a still-running process is
   not treated as a disconnect signal.
5. Consecutive start failures for the same device use bounded delays of 500 ms,
   1 s, 2 s, 4 s, and 5 s; later failures stay capped at 5 s. A target change or
   successful start resets the retry state.
6. A replacement stream cannot start until the old stream has completed Stop or
   reap. Fast Stop→Start therefore creates exactly one replacement.
7. The existing `stream_id` is the only batch fence. Manual Stop retains valid
   final batches for that stream; target replacement invalidates the old
   `stream_id` immediately and discards its late batches while still draining the
   bounded channel so the worker can finish.
8. Controller owns reconciliation. `TerminalApp` forwards user commands and
   renders Controller state; it does not own a second pending-intent state
   machine or timer.

## Capability boundary

Strict LiveOnly attach is unsupported until the supported HDC/HiLog matrix has a
verified server-side cursor, since boundary, or equivalent subscription
contract. ArkLog does not use host time as a device-log watermark and does not
clear the device-wide buffer. A LiveOnly start returns
`DeviceLogStartError::UnsupportedLiveStart` rather than silently falling back to
ordinary `hilog`.

## Acceptance evidence

- Cross-platform Controller tests cover persistent Running through no-device and
  start-failure states plus retry suppression.
- Fake-HDC tests cover discovery-driven start, refresh coalescing, Stop→Start,
  A→B replacement, late-batch fencing, final manual-Stop batches, unexpected
  exit/reap, and a full bounded channel during Stop.
- Pure policy tests cover the capped retry sequence; core tests cover the typed
  LiveOnly capability failure without launching HDC.

# Initial slice TDD evidence

Parent state: new repository with no history. ArkLine reference revision: `e8e91334ecc71db172be01d623c79cafb731987d`.

## React workbench

RED command: `pnpm test`

Observed failure: Vite could not resolve `../src/App` from `tests/app.test.tsx`.

GREEN command: `pnpm test`

Protected behavior: discover a device, start its stream, render an incoming batch, and stop the stream.

## HDC discovery

RED command: `cargo test -p arklog-core --test device_discovery`

Observed failure: unresolved import `arklog_core` because the core library target did not exist.

GREEN command: `cargo test -p arklog-core --test device_discovery`

Protected behavior: invoke verbose HDC target discovery and parse structured online/offline devices.

## Output batching

RED command: `cargo test -p arklog-core --test log_stream`

Observed failure: missing `spawn_log_reader`, `DeviceLogOutputBatch`, and `LogBatchSink`.

GREEN command: `cargo test -p arklog-core --test log_stream`

Protected behavior: flush the final partial batch without losing log lines.

## Process lifecycle

RED command: `cargo test -p arklog-core --test runtime`

Observed failure: unresolved import `arklog_core::DeviceLogRuntime`.

GREEN command: `cargo test -p arklog-core --test runtime`

Protected behavior: launch `hdc -t <device> hilog`, deliver output, reject an empty device id, stop, and reap the child.

## Tauri frontend boundary

RED command: `pnpm test`

Observed failure: Vite could not resolve `../src/tauri-api`.

GREEN command: `pnpm test`

Protected behavior: map the public frontend API to Tauri commands and forward `device-log-output` events.

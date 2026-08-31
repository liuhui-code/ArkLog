# Initial slice TDD evidence

Parent state: new repository with no history. ArkLine reference revision: `e8e91334ecc71db172be01d623c79cafb731987d`.

## Ratatui low-memory runtime

Specification: `docs/specs/ratatui-low-memory-runtime.md`.

RED commands and observed failures, one vertical slice at a time:

- focused session-store test: missing `SessionLogStore`; later slices exposed
  missing regex re-index, invalid-pattern, clear, and disk-backed find APIs.
- focused state tests: missing anchored scrolling, filter feedback, wrapped find,
  and shared HiLog/Fault Log tab behavior.
- focused TestBackend tests: missing compact shared workspace, regex highlight,
  and find highlight rendering.
- fake-HDC controller tests: missing bounded-channel stream and Fault Log host
  integration.
- focused memory-probe test: missing the 100,000-line RSS probe.

GREEN commands:

- `cargo test -p arklog`
- `cargo run -p arklog --release -- --memory-probe 100000`

Protected behavior: complete session lines live in temporary raw/index files;
heap use remains bounded; regex compilation is capped; stream transport uses
backpressure; filtering, find, clear, scrolling, device changes, tabs, and
Fault Log refresh work through public Rust interfaces; the release process
fails its gate at 50 MiB peak RSS.

Verification on 2026-08-30:

- macOS release, 100,000 raw lines plus 50,000 regex-visible and find-indexed
  lines: `peak_rss_bytes=2347008`, with filter rebuild `313 ms` and find rebuild
  `322 ms` on the local machine.
- `cargo check --target x86_64-pc-windows-msvc -p arklog`: passed. The same
  `memory:check` command uses the Windows process working-set API when run on a
  native Windows release host.

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

## In-memory regex filtering

Specification: `docs/specs/regex-log-filter.md`.

RED command: `pnpm test -- tests/app.test.tsx`

Observed failures, one vertical slice at a time: missing `Filter logs`, `Regex`,
`Match Case`, and inline `alert` elements.

GREEN command: `pnpm test -- tests/device-log-filter.test.ts tests/app.test.tsx`

Protected behavior: case-insensitive plain filtering, regex mode,
case-sensitive matching, invalid-pattern feedback, restoring buffered lines
after clearing the query, and skipping regex evaluation for log lines longer
than 4,096 characters.

## Structured in-memory filtering

Specification: `docs/specs/structured-log-filter.md`.

RED commands: focused `pnpm test -- tests/app.test.tsx -t <slice>` runs and
`pnpm test -- tests/device-log-entry.test.ts`.

Observed failures, one vertical slice at a time: missing level controls, PID
input and validation, process/domain/tag inputs, clear-filter control, and
timestamp/TID/message fields on the parsed entry.

GREEN command:
`pnpm test -- tests/app.test.tsx tests/device-log-filter.test.ts tests/device-log-entry.test.ts`.

Protected behavior: standard HiLog parsing and raw-line fallback; level OR
selection; exact numeric PID; process/domain/tag substring matching; AND
composition across categories; case handling; and restoring all buffered logs
when every filter is cleared. Existing text and regex behavior remains covered.

## SQLite log history

Specification: `docs/specs/sqlite-log-history.md`.

RED commands and observed failures:

- `cargo test -p arklog-core --test log_history`: missing public
  `DeviceLogStore`.
- `cargo test -p arklog-core --test log_history persists_a_live_batch`:
  missing `PersistingLogBatchSink`.
- focused `tests/tauri-api.test.ts`: `api.queryHistory is not a function`.
- focused `tests/app.test.tsx`: missing accessible `Load history` button.

GREEN commands:

- `cargo test -p arklog-core --test log_history`
- `pnpm test -- tests/tauri-api.test.ts tests/app.test.tsx`

Protected behavior: transactional batch persistence, database reopen, bounded
per-device recent history in display order, durable-live sink composition,
Tauri query mapping, history loading, and reuse of the existing filter chain.

## History cursor pagination

Specification: `docs/specs/history-cursor-pagination.md`.

RED commands and observed failures:

- focused `cargo test -p arklog-core --test log_history walks_older_history`:
  missing public `DeviceLogStore::page`.
- focused `tests/tauri-api.test.ts`: the Tauri invocation omitted
  `beforeSeq`.
- focused initial-history UI test: the old array contract failed with
  `rows.map is not a function` after introducing a page object.
- focused continuation UI test: missing accessible `Load older history`
  button.

GREEN commands:

- `cargo test -p arklog-core --test log_history`
- `pnpm test -- tests/tauri-api.test.ts tests/app.test.tsx`

Protected behavior: exclusive sequence cursors, limit-plus-one continuation
detection, oldest-to-newest page ordering, cursor mapping, immutable prepend,
no repeated boundary row, terminal-page handling, and the 2,000-row frontend
cap.

## Storage governance

Specification: `docs/specs/storage-governance.md`.

RED commands and observed failures:

- focused core retention test: missing
  `PersistingLogBatchSink::with_retention_limit`.
- focused core health test: missing `DeviceLogStore::health`.
- focused core clear test: missing `DeviceLogStore::clear`.
- focused `tests/tauri-api.test.ts` tests: missing `getStorageHealth` and
  `clearHistory` functions.
- focused health UI test: missing the stored-row pressure display.
- focused clear UI test: missing accessible `Clear history` button.

GREEN commands:

- `cargo test -p arklog-core --test log_history`
- `pnpm test -- tests/tauri-api.test.ts tests/app.test.tsx`

Protected behavior: global newest-row retention after successful writes,
downstream live delivery, aggregate health classification, transactional
table-only clear, Tauri boundary mapping, storage-pressure presentation, and
two explicit actions before destructive clearing.

## Fault Log MVP

Specification: `docs/specs/fault-log-mvp.md`.

RED commands and observed failures:

- focused core fetch test: missing `DeviceFaultLogStatus` and
  `HdcClient::list_fault_logs`.
- focused unavailable-state test: returned `Error` instead of `Unavailable`.
- focused unauthorized-state test: returned `Error` instead of
  `Unauthorized`.
- focused `tests/tauri-api.test.ts`: `api.listFaultLogs is not a function`.
- focused `tests/app.test.tsx`: missing accessible `Fault Log` tab.

GREEN commands:

- `cargo test -p arklog-core --test fault_log`
- `pnpm test -- tests/tauri-api.test.ts tests/app.test.tsx`
- `cargo check -p arklog`

Protected behavior: HDC command construction, raw-entry splitting with internal
blank-line preservation, structured ready/empty/unavailable/unauthorized/error
states, Tauri boundary mapping, HiLog/Fault Log tabs, refresh, initial
selection, row selection, and raw inspection.

## Fault Log filtering and copy

Specification: `docs/specs/fault-log-filter-copy.md`.

RED commands and observed failures:

- focused entry test: missing `DeviceFaultLogEntry.type`.
- focused taxonomy test: supported explicit reasons remained `unknown`.
- focused summary test: missing `DeviceFaultLogEntry.copySummary`.
- focused filter test: missing `device-fault-log-filter` module.
- focused regex test: invalid regex produced no validation error.
- focused PID test: non-numeric PID produced no validation error.
- focused oversized-input test: regex evaluated a raw entry over 4,096 chars.
- focused clipboard boundary test: `api.writeClipboard is not a function`.
- focused panel filter test: missing accessible `Fault log type` control.
- focused panel copy test: missing accessible `Copy Fault Summary` action.

GREEN commands:

- `pnpm test -- tests/device-fault-log-entry.test.ts tests/device-fault-log-filter.test.ts tests/fault-log-panel.test.tsx tests/tauri-api.test.ts`

Protected behavior: conservative type classification, stable summary format,
AND-composed type/PID/process/text matching, plain and regex modes, inline
validation, long-input protection, visible-selection fallback, host-boundary
clipboard writes, and exact raw copying.

## Fault Log file export

Specification: `docs/specs/fault-log-file-export.md`.

RED commands and observed failures:

- focused core export test: missing public `DeviceFaultLogExporter`.
- focused `tests/tauri-api.test.ts`: `api.exportFaultLogs is not a function`.
- focused `tests/fault-log-panel.test.tsx`: missing accessible
  `Export Visible Fault Logs` action.

GREEN commands:

- `cargo test -p arklog-core --test fault_log`
- `pnpm test -- tests/tauri-api.test.ts tests/fault-log-panel.test.tsx`
- `cargo check -p arklog`

Protected behavior: deterministic text formatting, selected-file replacement,
raw payload and visible-order preservation, byte and entry counts, Tauri save
dialog mapping, cancellation as a normal result, filtered-only export, and
success status without clearing the current inspector.

## Shared HiLog and Fault Log workspace

Specification: `docs/specs/shared-log-workspace.md`.

RED command and observed failure:

- focused `tests/app.test.tsx`: no accessible `tabpanel` existed because HiLog
  and Fault Log each created an independent outer `log-panel` section.

GREEN command:

- `pnpm test -- tests/app.test.tsx tests/fault-log-panel.test.tsx`

Protected behavior: one stable workspace controlled by both tabs, mutually
exclusive visible content, preserved view state across switches, and unchanged
HiLog and Fault Log behavior inside the shared area.

## Compact top controls

Specification: `docs/specs/compact-top-controls.md`.

RED command and observed failure:

- focused `tests/app.test.tsx`: the ArkLog heading and runtime status were
  outside `Stream controls`, and a standalone application banner remained.
- focused compact-height assertion: the rendered control region had no bounded
  height instead of the specified 48 pixels.
- focused merged-controls assertion: the visible ArkLog heading remained and
  the log tablist was outside `Stream controls`.
- focused empty-device assertion: `No devices` appeared twice beside a separate
  `CONNECTION / unavailable` state.

GREEN command:

- focused `pnpm test -- tests/app.test.tsx -t "keeps runtime status and log
  tabs inside the device control row"`.

Protected behavior: live status and HiLog/Fault Log tabs share one compact
48-pixel device-control region, no visible brand or standalone tab row consumes
space, empty discovery has one device state with no duplicate unavailable
meaning, and the shared log workspace retains the recovered vertical area.

Full-suite verification initially hit Vitest's default five-second timeout in
two interaction-heavy tests under parallel JSDOM load. Both files passed when
run independently. The explicit ten-second project timeout keeps the full gate
stable while preserving immediate assertion failures.

## Compact HiLog content

Specification: `docs/specs/shared-log-workspace.md`.

RED command and observed failure:

- focused shared-workspace test still found the visible `HiLog output` heading.

GREEN command:

- focused `tests/app.test.tsx` shared-workspace and older-history tests.

Protected behavior: HiLog filters begin at the top of the shared panel, buffer
count and older-history action remain available in the existing tab strip, and
Fault Log retains its own diagnostic heading.

## Stable live-log view

Specification: `docs/specs/live-log-view.md`.

RED commands and observed failures:

- focused `DeviceLogFilter` test: a matching raw line longer than 4,096
  characters was rejected.
- focused public UI test: the filter input escaped regular-expression syntax
  until a separate Regex checkbox was enabled.
- focused `DeviceLogBuffer` test: the no-eviction domain object did not exist.
- focused public UI test: the HiLog viewport had no accessible scroll region or
  `Back to latest` action.

GREEN commands:

- `pnpm exec vitest run tests/device-log-filter.test.ts`
- `pnpm exec vitest run tests/device-log-buffer.test.ts`
- focused `tests/app.test.tsx` regular-expression, clear-view, and follow-latest
  tests.

Protected behavior: exact untruncated raw lines, regex-only HiLog filtering,
immutable no-eviction buffering, explicit view clearing, scroll-away without
forced jumping, and one-action return to the latest line.

## Live-only HiLog architecture

Specification: `docs/specs/live-log-view.md`.

RED commands and observed failures:

- focused public UI test found the visible `History` action and SQLite storage
  controls.
- focused Tauri API test found `queryHistory`, `getStorageHealth`, and
  `clearHistory` on the public boundary.

GREEN commands:

- focused `tests/app.test.tsx` live-only controls test.
- focused `tests/tauri-api.test.ts` live-only boundary test.
- focused frontend tests for `App`, `createTauriArkLogApi`, and
  `DeviceLogBuffer`.

Protected behavior: HiLog batches travel directly from the runtime sink to the
Tauri event and remain only in the current React session; no SQLite dependency,
history command, retention state, or persisted-history UI remains.

## Conventional in-view HiLog find

Specification: `docs/specs/in-view-log-find.md`.

RED commands and observed failures:

- focused public UI test could not find a `Find in HiLog` input after
  `Ctrl+F`.
- focused navigation test remained at result `1 / 2` after Enter.
- focused close test left the find bar open when Escape was pressed outside
  its input.

GREEN commands:

- focused `tests/app.test.tsx` conventional-find test.
- focused `tests/app.test.tsx` navigation-and-close test.
- combined `tests/app.test.tsx` and `tests/device-log-find.test.ts` regression.

Protected behavior: `Ctrl+F` and `Command+F` open and focus literal,
case-insensitive find; matches are highlighted and counted; Enter and
Shift+Enter wrap through results; Escape closes find without changing logs or
the regular-expression filter.

## Regular-expression match highlighting

Specification: `docs/specs/regex-match-highlighting.md`.

RED commands and observed failures:

- focused public UI test found the correctly filtered raw line but no
  `mark.regex-match` elements for its two full matches.
- focused overlap test found only `find-match is-current`; the underlying
  regular-expression decoration had been replaced.

GREEN commands:

- focused `tests/app.test.tsx` all-regex-matches test.
- focused `tests/app.test.tsx` overlapping-decoration test.
- combined filter, highlighter, and public UI regression tests.

Protected behavior: every non-empty full match is highlighted without changing
raw text; zero-width matches do not create empty marks; invalid expressions
remain safe; regex and in-view find decorations compose through lossless,
non-overlapping presentation segments.

## Smooth lossless live streaming

Specification: `docs/specs/smooth-lossless-streaming.md`.

RED commands and observed failures:

- the 10,000-line public UI pressure test mounted all 10,000 rows and took about
  28 seconds.
- the start-race test lost the matching first batch emitted before
  `startStream` returned.
- the delayed-subscription test allowed streaming before its output listener
  existed.
- the pending start and stop tests left their command controls enabled.
- the runtime stop test returned before `tail-before-stop` was delivered.
- the session find-index test failed because no incremental find boundary
  existed.

GREEN commands:

- focused `tests/app.test.tsx` 10,000-line pressure, paused-scroll, and offscreen
  find test; the pressure slice completes in about 2.5 seconds and mounts at
  most 200 rows.
- focused `tests/app-stream-lifecycle.test.tsx` lifecycle race suite.
- focused `tests/device-log-session.test.ts` 100,000-line, filter, clear, and
  incremental-find suite.
- focused `tests/device-log-virtual-window.test.ts` range-boundary suite.
- isolated-target `arklog-core` runtime stop-tail and 10,037-line ordered-burst
  tests.

Protected behavior: no intentional in-memory eviction, ordered large bursts,
one repaint notification per frame, bounded DOM work, stable paused scrolling,
offscreen find, listener-before-start ordering, single-flight lifecycle
commands, and worker-joined final-batch delivery.

## Truthful device refresh and Ctrl commands

Specification: `docs/specs/ratatui-low-memory-runtime.md`.

RED commands and observed failures:

- the public controller test returned `Stop HiLog before refreshing devices`
  while a normal active stream was running.
- HDC discovery failure was silently reduced to `No devices`, and startup
  replaced the original daemon error with the same generic message.
- the public TestBackend view had no complete device collection or connected
  device list mode.
- the keymap contract could not import a command mapper, while the application
  handled bare `Q`, `S`, `R`, `/`, `G`, `C`, and `N` characters directly.
- a disconnect refresh left the removed device's stream marked `Streaming`,
  and an offline selected device still rendered a green connection marker.
- a successful 30 ms background batch poll erased the last discovery or
  refresh error before the user could read it.

GREEN commands:

- focused `controller_stream`, `ui`, and `keymap` integration tests.
- `pnpm test`, `pnpm build`, `cargo test --workspace`, and
  `cargo check -p arklog`.
- `pnpm memory:check` and the Windows MSVC target check.

Protected behavior: device refresh works during an active stream without
interrupting a still-online selected device; confirmed disconnects stop stale
streams; HDC errors remain visible in the single no-device status; `Ctrl+D`
opens a complete device list; and all character commands require Control.
Successful background polling never clears the last user-action error.

## Truthful state and bounded UI work

RED: official verbose HDC rows disappeared; stale Fault results crossed devices; async stop lost its tail; exited HDC stayed `LIVE`; idle pumping redrew globally; query changes rescanned raw logs twice; a slow UI buffered megabytes.

GREEN: focused discovery, controller stream/Fault, UI, store, runtime, and slow-consumer tests now pass.

Protected behavior: displayed connection/stream state follows reality, final logs survive, redraw and per-tick work are bounded, unchanged queries do zero work, and filter/find rebuilds avoid duplicate scans.

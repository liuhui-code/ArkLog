# Initial slice TDD evidence

Parent state: new repository with no history. ArkLine reference revision: `e8e91334ecc71db172be01d623c79cafb731987d`.

## Tokenized compact Ratatui workspace

Specification: `docs/specs/terminal-visual-theme.md`.

RED commands and observed failures, one vertical slice at a time:

- focused `ui` render test: HiLog began on row 8 instead of row 5 and ended above a two-row footer;
- focused `theme_ui` test: Fatal styling colored and bolded the entire log row instead of only `F`;
- focused `stream_status_ui` test: removing the footer made the last action error invisible;
- focused token-ownership test: `ui.rs` and `input_ui.rs` still constructed styles and owned raw layout values;
- focused optional-line-number test: a prefixed row number prevented the real HiLog Level field from receiving its semantic style;
- focused status-migration test: visible/raw counts and Find position disappeared with the footer.

GREEN commands:

- `cargo test -p arklog --test ui`
- `cargo test -p arklog --test theme_ui`
- `cargo test -p arklog --test stream_status_ui`
- `cargo test -p arklog --test ui_input`

Protected behavior: a single three-row top control band and no footer maximize the full-width
HiLog viewport; errors and counts remain visible without consuming content rows; only the parsed
Level field is level-colored; optional line numbers and all other fields remain neutral; active
regex/Find matches are local; current rows use background only; and all visual styles and
design-owned dimensions come from the theme layer.

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
- official hidumper output/error tests: legacy command and false `Ready`.
- focused `tests/tauri-api.test.ts`: `api.listFaultLogs is not a function`.
- focused `tests/app.test.tsx`: missing accessible `Fault Log` tab.

GREEN commands:

- `cargo test -p arklog-core --test fault_log`
- `pnpm test -- tests/tauri-api.test.ts tests/app.test.tsx`
- `cargo check -p arklog`

Protected behavior: official hidumper construction, delimited raw-entry
splitting, structured ready/empty/unavailable/unauthorized/error
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
opens devices; bounded inputs support standard editing; `Ctrl+Q` exits safely.
Successful background polling never clears the last user-action error.

## Truthful state and bounded UI work

RED: official HDC rows disappeared; stale Fault results crossed devices; async stop lost its tail; exited HDC stayed `LIVE`; pending stop rendered contradictory status; refresh completion overrode cancelled start intent; idle work redrew globally; queries rescanned raw logs twice.

GREEN: focused discovery, controller stream/Fault, stream-intent/UI, store, runtime, and slow-consumer tests now pass.

Protected behavior: lifecycle display follows confirmed state while queued intent stays separate; cancellation survives refresh; final logs survive; redraw and per-tick work stay bounded; unchanged queries do zero rebuild work.

## Windows Terminal log-view navigation (2026-09-28)

RED observed at the real terminal key-dispatch entry:

- `shift_tab_switches_between_the_log_views` stayed on HiLog after a BackTab
  event instead of switching to Fault Log.
- `function_keys_select_log_views_even_while_editing_or_browsing_devices`
  stayed on HiLog when F2 was pressed from the regex editor.
- The minimum-width TestBackend view lacked a visible F1/F2 shortcut hint.

GREEN: those focused tests pass after supporting BackTab, direct F1/F2 view
selection, and the compact header hint. Public keymap tests additionally check
all view/overlay contexts and leave modified function keys unclaimed. These
tests are not Unix-gated and cover repeated Windows-style Press events as well
as Repeat events without reversing the selected view. Session logs, the applied
filter, and Find remain intact across view selection.

Verification on macOS after the navigation slice:

- Focused binary/keymap/clear-UI/theme/input/render tests passed.
- `cargo test --workspace`, `cargo check -p arklog`, `pnpm test`, and
  `pnpm build` passed.
- `pnpm memory:check` passed with 100,000 lines and peak RSS 12,849,152 bytes,
  below the 52,428,800-byte budget.
- `cargo check -p arklog --target x86_64-pc-windows-msvc --tests` passed. This
  compiles the cross-platform event-dispatch regressions for Windows; it does
  not claim a native Windows Terminal interactive test.
- `cargo fmt --all --check` and `git diff --check` passed.

## Windows Ctrl+L freeze diagnosis (2026-10-11)

The reported failure is Windows Ctrl+L: the displayed count becomes zero and
all keyboard commands stop responding. A fake-HDC Clear regression on macOS
confirms that Clear preserves the live stream, restores following-latest, and
displays output sent after Clear. It does not reproduce the complete UI freeze.

RED/GREEN diagnostic slices cover Clear completion at the Ctrl+L dispatch entry
and ordered start/end events for the next redraw. The trace now distinguishes
the synchronous clear operation from terminal sizing, session-window reading,
and frame drawing, without recording log content or adding per-poll diagnostic
traffic. A Windows execution trace remains needed to identify the original
blocked operation; this freeze is not claimed fixed.

Verification: focused Clear/key-dispatch/diagnostics tests, `pnpm test`,
`pnpm build`, `cargo test --workspace`, `cargo check -p arklog`, Windows MSVC
test compilation, `cargo fmt --all --check`, and `git diff --check` passed.
The Windows check compiles tests; it does not run a Windows terminal session.

## v0.2.8 release integration (2026-10-11)

Integrated all local changes with the v0.2.7 remote main branch, preserving
long-line navigation, soft wrap, query history, device phone codes, and the
post-disconnect retry path. Conflict resolution keeps both the phone-code and
successful-exit daemon-failure discovery regressions, plus both navigation
keymap suites. Focused discovery, terminal dispatch, Clear, diagnostics, and
stream-controller tests pass on the combined code.

RED: `release_package_version_is_v0_2_8` failed with package version 0.2.7
before editing package metadata. GREEN: the complete release-workflow test
passes after setting the package.json, Rust package, and lockfile to 0.2.8.

Final local verification: `pnpm test`, `pnpm build`, `cargo test --workspace`,
`cargo check -p arklog`, Windows MSVC test compilation, `cargo fmt --all
--check`, and `git diff --check` pass. `pnpm memory:check` passes with 100,000
lines and peak RSS 13,279,232 bytes below the 52,428,800-byte budget.

## Native Windows console selection freeze (2026-10-30)

Reproduced over `ssh windows-pc` on Windows 10 build 19045, including the
logged-in desktop session with a real mouse drag in a dedicated console.
The unmodified v0.2.8 binary retains input mode `0x1f0` after entering raw
mode: Quick Edit is still enabled. Selecting text suspends console I/O, so
Ctrl+L, Ctrl+D, and Ctrl+Q appear completely unresponsive. Escape cancels the
host selection, immediately releasing the queued application commands.
This reproduces independently of HDC or device log throughput.

RED through the actual Windows executable, before changing production code:

- `native_console_keeps_shortcuts_responsive_and_restores_quick_edit` fails
  because the running console still has `ENABLE_QUICK_EDIT_MODE` (`0x40`).
- `failed_terminal_initialization_restores_console_input_mode` fails after
  closing the terminal output pipe: the original binary leaves input mode
  `496` instead of the original `503`.

GREEN: a Windows-only session guard saves input mode before Ratatui
initialization, disables Quick Edit with `ENABLE_EXTENDED_FLAGS`, and restores
the original input mode after teardown or an error. Both native binary
regressions pass in isolated consoles, including headless SSH execution.
The interaction regression injects real console key records, waits for Clear's
completed frame, checks the device command, quits, and checks restoration.

Desktop differential loop on the same Windows machine, using the same real
mouse drag and Ctrl+L/D/Q sequence: the original freezes 3/3 times and resumes
only after Escape; the fixed release freezes 0/10 times and exits normally.
The running fixed input mode is `0x1b0`, differing only by the Quick Edit bit.
Classic-console automatic drag selection is disabled during ArkLog; Windows
Terminal remains the recommended host for mouse selection and copying.

Final verification on both macOS and native Windows: `pnpm test`, `pnpm build`,
`cargo test --workspace`, and `cargo check -p arklog` pass. The complete Rust
workspace tests and checks pass again after adding the initialization-error
regression. `cargo fmt --all --check` and `git diff --check` pass.

## Regex field stability during device refresh (2026-10-30)

RED: `automatic_device_refresh_never_replaces_the_applied_regex` renders the
public UI through repeated ready/refreshing cycles. On the original code,
`Refreshing devices` replaces both the regex title and the applied expression
with a red connection message, even while the selected device remains online
and HiLog is streaming.

GREEN: routine refreshing is treated as a normal connection state when
rendering the query field. The regression checks that the applied expression
and complete top controls remain identical across repeated refresh cycles.
Existing connection-failure, action-error, input-editing, and stream-status
rendering tests also pass.

Verification on macOS and native Windows: focused rendering regressions,
`pnpm test`, `pnpm build`, `cargo test --workspace`, and `cargo check -p arklog`.
The Windows executable includes both this fix and the console-freeze fix above.

## v0.2.9 release (2026-10-30)

RED: `release_package_version_is_v0_2_9` fails against the existing 0.2.8
metadata. GREEN: the release-workflow suite passes after updating the Rust
package, Cargo lockfile, and package.json to 0.2.9.

The release includes the Windows console-freeze and regex-refresh fixes above.
Release validation runs `pnpm test`, `pnpm build`, `cargo test --workspace`,
`cargo check -p arklog`, and `pnpm memory:check` on macOS and native Windows,
followed by the existing three-platform GitHub Actions release workflow.

## AI Agent MCP queries

Specification: `docs/specs/agent-log-query.md`.

RED → GREEN vertical slices:

- `cargo test -p arklog --test agent_query`: missing `LogQuery` / snapshot API;
  added raw-session snapshots and inclusive device-time + regex queries.
- Pagination test: missing cursor/session/until fields; added fixed-boundary
  pages, independent readers and shared directory lifetime across Clear.
- `cargo test -p arklog --test mcp`: missing stdio entry, then missing runtime;
  added negotiation/discovery and bounded loopback query integration.
- Binary MCP test: attempted terminal initialization; added the early `--mcp`
  path and the opt-in runtime hook in the TUI loop.
- Sparse query test: scanned through the entire 12 MB history; added scan-byte
  and time budgets with resumable raw cursors.
- Invalid initialization test: accepted missing protocol parameters; added
  initialization parameter validation.
- Binary configuration test: accepted port 0, leaving no connectable configured
  endpoint; reject configured ports outside 1–65535 before starting.

Focused GREEN: `cargo test -p arklog --test agent_query --test mcp`.
Loopback integration tests require permission to bind a local ephemeral port.

Final verification (macOS, 2026-11-01): `pnpm test`, `pnpm build`,
`cargo test --workspace` (164 passing tests), `cargo check -p arklog`,
`cargo fmt --all --check`, and `git diff --check` passed. The release
`pnpm memory:check` passed with 100,000 records and 13,103,104 bytes peak RSS
against the 52,428,800-byte budget. A real pseudo-terminal TUI + fake-HDC smoke
queried `first` through the binary's MCP stdio adapter, reported device USB-01
and two raw records, then exited cleanly with Ctrl+Q.

## Optional Unix timestamps and boolean MCP query

RED → GREEN vertical slices:

- Existence test: missing `LogExistsQuery` / `has_logs`; added complete-snapshot
  boolean scans that resume beyond the 100,000-record page boundary.
- Time query test: integer `start`/`end` did not compile with the string API;
  changed both to optional signed Unix millisecond integers and parsed device
  epoch seconds using exact integer arithmetic.
- Strict-after test: missing `after`; added optional millisecond `after` with
  an exclusive lower bound, while ordinary query bounds stay inclusive.
- Fake-HDC runtime test: captured `hilog` without format arguments; changed the
  command to `hilog -v epoch` so timestamps need no guessed year/timezone.
- MCP discovery test: only two tools and string time schemas; added optional
  integer time schemas and the `arklog_has_logs` boolean output schema.
- MCP integration test: runtime rejected the new tool; connected it to the
  snapshot existence query, including true/false, omitted time and validation.
- Epoch render test: Level was neutral; taught field parsing the epoch prefix
  while leaving every other field neutral.

Focused verification: `cargo test -p arklog --test agent_query --test mcp
--test theme_ui --test controller_stream` and the core runtime test.

Final follow-up verification (macOS, 2026-11-01): `pnpm test` and
`cargo test --workspace` each passed all 169 tests; `pnpm build`,
`cargo check -p arklog`, formatting and diff checks passed. One earlier broad
run hit the existing 3-second background-reap timeout; its isolated rerun and
both final full runs passed without changing that test's timing policy.
`pnpm memory:check` passed 100,000 records at 12,992,512 bytes peak RSS
against the 52,428,800-byte budget. A real pseudo-terminal TUI + epoch fake-HDC
smoke verified inclusive millisecond range queries, strict-after true/false,
regex-only queries without time, and a clean Ctrl+Q exit.

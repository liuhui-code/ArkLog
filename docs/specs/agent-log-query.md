# Agent HiLog query / MCP

Status: Implemented.

The production TUI remains the collector. `ARKLOG_MCP_PORT` opts into a
loopback-only, bounded, newline-JSON IPC listener. `arklog --mcp` exposes the
standard MCP stdio transport and forwards tool calls to that running process.
Initialization, ping and tool discovery work without HDC or a running TUI;
queries against an absent runtime return `isError: true`. No mutation tools,
second collector, persistent log database or HTTP transport are added.

## Tools and query contract

- `arklog_session`: session ID, selected device ID (nullable), raw record count.
- `arklog_query_logs`: optional inclusive `start` / `end` as signed Unix
  millisecond integers; optional case-insensitive Rust `regex`; `limit` (1–1000,
  default 100); `cursor` (raw record ordinal, default 0); `session_id`; `until`.
  No time arguments are required; either bound can be supplied independently.
- The collector runs `hdc -t <device> hilog -v epoch`. HiLog prints device Unix
  seconds with millisecond precision; integer arithmetic converts that prefix
  to Unix milliseconds. No host timezone/year or ingestion time is substituted.
  Time ranges may cross years; inverted bounds and noninteger input fail.
  Unparseable timestamps are excluded only when a time bound is present.
- `arklog_has_logs`: required case-insensitive `regex`, optional Unix millisecond
  `after`. Returns structured `{ "exists": boolean }`, without log text. With
  `after`, only timestamps strictly greater than the bound qualify; without it,
  all raw records (including untimestamped ones) are eligible. It scans a fixed
  snapshot through all pages before returning false, stops at the first match,
  and is independent of UI filters and query response-text limits. A 5-second
  scan budget is checked between pages; incomplete/error scans return tool errors
  rather than false. It does not wait for future appends.
- Results preserve complete raw text and capture order, independently of TUI
  filter/Find. No early termination based on timestamp order is allowed.
- Log query results include `session_id`, `device_id`, `raw_count`, `until`, `records`
  (`index`, `text`) and nullable `next_cursor`. Repeat filters and carry the
  first page's session ID and exclusive `until` raw count to freeze pagination.
  A nonzero cursor requires the current session ID. A stale ID fails explicitly.
- Clear/device changes/stream restarts (including reconnects) replace the
  session. Already opened snapshots can finish;
  new requests cannot query the old session. Exit closes the IPC listener.
- Source regex is capped at 4096 bytes; compiled regex/DFA caches at 1 MiB each.
- Per-page scans stop at 100,000 records, about 8 MiB or a 250 ms budget checked
  between records. Empty pages can carry a cursor. These are bounded scan slices,
  not a hard latency guarantee for filesystem reads or individual regex matches.
- Query result raw text is capped at 256 KiB, without dropping a boundary record.
  A single matching record above this cap or an input record above 1 MiB returns
  a tool error instead of truncating or allocating unbounded text. Existence
  queries retain no matched text and may match records above the 256 KiB response
  limit, but the 1 MiB input-record bound still applies.

## Runtime boundary

The UI loop handles only a capacity-one snapshot request/reply channel. A
snapshot flushes the store and opens independent read handles at a fixed raw
count. Scanning never holds a UI/state mutex or mutates visible/find indexes.
The store and snapshots share a directory lifetime guard, so Clear cannot delete
files still in use (including on Windows). One worker accepts one IPC request at
a time, with socket and snapshot-reply timeouts, bounded JSON frames (16 KiB
requests, 8 MiB responses), and shutdown on runtime drop. Local processes can
read logs when explicitly enabled; no remote binding is accepted.

MCP requests use JSON-RPC 2.0, newline-delimited stdio; stdout contains only MCP
messages. `initialize` negotiates a supported protocol version and advertises
only tools. Notifications receive no replies. Unknown methods/tools use protocol
errors; query and runtime failures use tool errors. Results include both text
JSON and `structuredContent`. Tool descriptions identify returned logs as
untrusted data and describe timestamp, regex and pagination rules.

References: [HiLog CLI](https://github.com/openharmony/hiviewdfx_hilog/blob/master/README.md),
[epoch prefix implementation](https://github.com/openharmony/hiviewdfx_hilog/blob/master/frameworks/libhilog/utils/log_print.cpp),
[MCP stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports),
[lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle),
[tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).

## Acceptance evidence

`tests/agent_query.rs` covers time/regex intersection and boundary inclusion,
UI-filter independence, frozen pagination during append, stale sessions after
Clear, old-snapshot completion, bounded sparse scans, validation and byte-boundary
pagination. `tests/mcp.rs` covers stdio negotiation/discovery, invalid protocol
parameters, actual loopback calls into a controller, regex errors, unavailable
runtime, listener shutdown, and the binary's terminal-free `--mcp` mode.

Timestamp/boolean follow-up tests additionally cover optional start/end, exact
millisecond comparisons around the Unix epoch, strict `after` boundaries,
regex-only existence, matches beyond the first scan page, no-match completion,
large matching records, rejected string timestamps, MCP boolean schema/results,
HDC epoch arguments and neutral epoch fields with only Level colored.

# History cursor pagination specification

Status: Superseded by `live-log-view.md`; ArkLog no longer exposes history.

Source behavior: ArkLine Device Log at revision
`e8e91334ecc71db172be01d623c79cafb731987d`.

## Goal

Let a user continue from the most recent stored device logs into older history
without offset drift or duplicate rows.

## Public behavior

1. A history request contains device ID, page limit, and optional `beforeSeq`.
2. `beforeSeq` is exclusive: a continuation page contains only rows whose
   sequence is lower than the cursor.
3. A page returns at most 500 rows in oldest-to-newest display order and a
   `nextCursorSeq` only when older rows remain.
4. The first `Load history` request has no cursor and replaces the current
   buffer with the newest page.
5. `Load older history` requests the returned cursor and prepends the next page
   without duplicating the boundary row.
6. The current regular expression continues to apply to the combined
   historical snapshot.
7. The frontend does not evict an older loaded row. Continuation remains
   available while the backend returns a cursor.
8. Starting a live stream clears the history cursor and preserves existing
   live-stream semantics.

## Object boundaries

- `DeviceLogStore` owns cursor interpretation, page bounds, ordering, and
  determination of whether older rows remain.
- `DeviceLogHistoryPage` is the stable core/Tauri/frontend transfer object.
- `ArkLogApi` maps the optional cursor without SQLite knowledge.
- `App` owns initial replacement versus continuation prepend behavior.

## Cursor rules

The store queries `limit + 1` rows ordered by descending sequence. The extra
row proves that another page exists. Returned rows exclude that probe row, are
reversed into display order, and use the oldest returned sequence as the next
exclusive cursor.

## Non-goals

- offset pagination
- server-side text or structured filtering
- retention, clear history, storage-health, or disk-pressure controls
- background prefetch or infinite scrolling
- query cancellation or deadlines

## Acceptance evidence

- A core integration test walks all pages and proves ordering and exclusive
  boundaries.
- A frontend boundary test proves optional cursor command mapping.
- A public UI test loads a second page, proves prepend order, and proves the
  continuation cursor is used exactly once.

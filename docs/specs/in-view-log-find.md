# In-view HiLog find specification

Status: Accepted.

## Goal

Let a user locate text in the currently displayed HiLog output with the
conventional find shortcut, without changing the log stream or regular
expression filter.

## Public behavior

1. `Ctrl+F` and `Command+F` open an in-view find bar and focus its query field.
2. Find treats the query as case-insensitive literal text, not as a regular
   expression.
3. Find searches only HiLog lines currently visible after regular-expression
   filtering. It never hides, truncates, or rewrites a log line.
4. Every match is highlighted, the current match is distinguished, and the
   find bar reports the current and total match counts.
5. `Enter` moves to the next match and `Shift+Enter` moves to the previous
   match. Navigation wraps at both ends and scrolls the current line into view.
6. `Escape` or the close action closes the find bar without removing logs or
   changing the regular-expression filter.
7. While the user navigates find results, follow-latest is paused. The existing
   `Back to latest` action remains the explicit way to resume live following.

## Object boundaries

- `DeviceLogFind` owns literal matching, occurrence ordering, and wrapped
  navigation.
- `DeviceLogFindBar` owns find controls and keyboard interactions inside the
  bar.
- `App` owns the global shortcut, current result selection, highlighting, and
  viewport positioning.

## Non-goals

- replacing the regular-expression filter
- searching buffered lines excluded by the active regular-expression filter
- searching Fault Log content
- persisting find state across application restarts

## Acceptance evidence

- A public UI test proves the conventional shortcut opens and focuses find,
  literal matches are highlighted, and counts are reported.
- A public UI test proves forward/backward wrapped navigation and closing.
- Unit tests cover literal matching and wrapped navigation edge cases.

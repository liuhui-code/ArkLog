# Regular-expression match highlighting specification

Status: Accepted.

## Goal

Make the exact substrings selected by the active HiLog regular-expression
filter immediately visible without changing any raw log text.

## Public behavior

1. Every non-empty full match of the active regular expression is highlighted
   in each visible HiLog line.
2. Matching is case-insensitive by default and otherwise uses the exact regular
   expression entered in `Filter logs`; users do not need to add `(?i)`.
3. A line may contain multiple highlighted matches, ordered from left to right.
4. Highlighting splits presentation text only. Copyable text content and the
   in-memory raw line remain byte-for-byte unchanged.
5. An empty expression displays every line without filter highlighting.
6. An invalid expression displays the existing inline error and no log lines or
   highlights.
7. A zero-width match may make a line visible but creates no empty visual mark.
8. Filter highlights and `Ctrl+F` highlights can overlap. Both decoration
   classes remain present on the overlap, with the current find result receiving
   the strongest visual emphasis.

## Object boundaries

- `DeviceLogFilter` owns expression validation, boolean matching, and all
  non-empty full-match ranges.
- `DeviceLogHighlighter` combines independently computed decoration ranges and
  emits non-overlapping presentation segments.
- React renders semantic `mark` elements from those segments. It never injects
  HTML or modifies the source string.

## Practice references

- MDN documents global `RegExp.exec()` iteration for successive matches and
  warns that zero-width matches require explicit progress.
- MDN identifies `mark` as the semantic element for text relevant to a user's
  current search operation.
- The CSS Custom Highlight API provides range-based decoration without DOM
  changes, but its Baseline 2025 support is too new for ArkLog's system-WebView
  compatibility target. ArkLog keeps the same range/decorations model while
  rendering broadly supported semantic `mark` elements.

## Acceptance evidence

- A public UI test proves all full matches are highlighted and raw line text is
  unchanged.
- A public UI test proves filter and in-view find highlights compose when their
  ranges overlap.
- Domain tests prove multiple-match ranges, zero-width handling, invalid
  expressions, and decoration segmentation.

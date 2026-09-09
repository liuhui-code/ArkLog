# ArkLog terminal visual theme

## Goal

Provide a readable, calm dark interface for continuous log inspection on Windows and
macOS without changing log content, increasing retained data, or requiring patched fonts.

## Host font boundary

Ratatui renders terminal cells and cannot choose the font used by the host terminal.
ArkLog therefore:

- uses only standard Unicode box-drawing and geometric characters;
- does not require Nerd Font or Powerline glyphs;
- documents no-ligature monospace font recommendations for exact log and regex reading;
- never emits styling characters into stored or copied raw log text.

## Palette

The single built-in theme follows the Catppuccin Mocha semantic palette:

- base `#1e1e2e`, text `#cdd6f4`;
- surface `#313244`, border `#6c7086`, muted `#9399b2`;
- accent `#cba6f7`, error `#f38ba8`, warning `#f9e2af`;
- success `#a6e3a1`, information `#89b4fa`.

The whole frame receives the base foreground and background once. Widgets then apply only
semantic overrides. This keeps blank cells coherent and avoids component-specific colors.

## Log semantics

- Only the parsed Level field receives a log-level style: fatal is red and bold;
  error is red; warning is yellow; info is green; debug is blue; verbose is muted.
- Time, optional line number, PID/TID, Tag, Message, and unclassified content use
  the neutral text color. A level never colors or bolds the whole row.

Filter matches use base text on a yellow background and bold weight. Find matches use the
accent foreground and underline. The current find line uses the surface background. Styles
compose so overlapping filter/find ranges remain visible without changing the raw string.
Only actual characters matched by an active regex/filter or non-empty Find query receive those
overrides. Current-match and selected rows add only the low-contrast surface background, so
unrelated field foregrounds remain intact.

## Layout

- Device state, HiLog/Fault Log navigation, regex/Find, and stream state share one compact
  three-row top control band.
- There is no bottom status bar. HiLog counts and Find position live in the existing workspace
  border title without consuming a content row.
- HiLog always receives the full workspace width and has no Details inspector. Fault Log keeps
  its entry list and raw-diagnostic inspector.

## Design-token ownership

`crates/arklog-tui/src/theme.rs` is the only owner of colors, styles, borders, selected and match
states, and layout dimensions or spacing that are visual design decisions. Rendering modules
consume semantic styles and named dimensions; they do not construct raw RGB values, modifiers,
styles, or unnamed `Length`/`Percentage` values. Structural layout semantics such as
`Constraint::Fill(1)` remain local and are not mechanically tokenized.

## Interaction semantics

- selected tabs use accent, bold, and underline;
- live/online states use success;
- stopped/offline states use warning;
- failures use error;
- shortcuts and secondary descriptions use muted text;
- selected lists use an information accent and surface background.

## Performance constraints

The theme is a fixed set of `Color` and `Style` values. Rendering performs no theme lookup,
configuration parsing, additional regex scan, or retained log allocation. Existing bounded
viewport rendering and the 50 MiB process memory gate remain authoritative.

## Verification

Public `render_app` tests assert the compact full-height workspace, absence of a footer, canvas,
border glyph and color, field-scoped semantic log levels, filter/find composition, low-contrast
current rows, selected tab, stream status, and secondary text. A source-boundary regression test
keeps visual values in the design-token layer. Existing UI tests continue to assert that raw log
text is unchanged.

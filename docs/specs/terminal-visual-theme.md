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

- fatal: red and bold;
- error: red;
- warning: yellow;
- info: green;
- debug: blue;
- verbose: muted;
- unclassified: normal text.

Filter matches use base text on a yellow background and bold weight. Find matches use the
accent foreground and underline. The current find line uses the surface background. Styles
compose so overlapping filter/find ranges remain visible without changing the raw string.

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

Public `render_app` tests assert the canvas, border glyph and color, semantic log levels,
filter/find composition, selected tab, stream status, and secondary text. Existing UI tests
continue to assert that raw log text is unchanged.

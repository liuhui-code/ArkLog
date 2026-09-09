# ArkLog contributor instructions

Every behavior change, defect fix, refactor, build change, and developer-tooling change uses test-driven development.

- Observe RED through the closest stable public interface before editing production code.
- Implement one minimal vertical slice at a time and keep tests GREEN while refactoring.
- Start defect fixes with a regression test.
- Run the focused test after each slice.
- Before handoff, run `pnpm test`, `pnpm build`, `cargo test --workspace`, and `cargo check -p arklog`.
- Keep `arklog-core` independent of Tauri. Host-specific command and event code belongs in `src-tauri`.
- Keep React components dependent on `ArkLogApi`; direct Tauri calls belong only in `src/tauri-api.ts`.
- The production UI is the Rust + Ratatui TUI. Do not restore React/Tauri as the production path.
- Keep every visual color, style, border, selection/match state, and design-owned layout dimension or spacing in `crates/arklog-tui/src/theme.rs`, exposed to renderers with semantic names. Structural constraints such as `Constraint::Fill(1)` do not need cosmetic tokens.
- HiLog uses the full workspace width with no Details pane. Only the parsed Level field may use level color; all other fields stay neutral except actual active regex/filter or Find match characters. Current-match and selected rows use only the low-contrast selection background.

# ArkLog contributor instructions

Every behavior change, defect fix, refactor, build change, and developer-tooling change uses test-driven development.

- Observe RED through the closest stable public interface before editing production code.
- Implement one minimal vertical slice at a time and keep tests GREEN while refactoring.
- Start defect fixes with a regression test.
- Run the focused test after each slice.
- Before handoff, run `pnpm test`, `pnpm build`, `cargo test --workspace`, and `cargo check -p arklog`.
- Keep `arklog-core` independent of Tauri. Host-specific command and event code belongs in `src-tauri`.
- Keep React components dependent on `ArkLogApi`; direct Tauri calls belong only in `src/tauri-api.ts`.

# Facts: TS/JS Dependency Update

- The goal updates only JS/TS dependencies in `package.json`. Rust dependencies in `src-tauri/Cargo.toml` are not touched.
- Safe updates (minor/patch bumps) are applied first as a single batch. These are: `@tauri-apps/api` → 2.11.0, `@tauri-apps/plugin-opener` → 2.5.4, `react` → 19.2.6, `react-dom` → 19.2.6, `@biomejs/biome` → 2.4.15, `@tauri-apps/cli` → 2.11.1.
- Major version jumps are applied one at a time, in order by risk: (1) `@vitejs/plugin-react` 4 → 6 and `vite` 7 → 8 together, (2) `vitest` 3 → 4 and `@vitest/coverage-v8` 3 → 4 together, (3) `jsdom` 26 → 29, (4) `typescript` 5 → 6.
- After each update or batch, the full quality gate runs: `just check`, `just lint`, `bun run test:run`.
- After each major version jump, the dev server is also verified to start (`bun run dev` starts without error, or `vite build` succeeds for CI-safe check).
- If verification fails for an update, that update is rolled back to the previous working state in a separate commit, and a `blocked.md` note is written documenting the reason.
- The final state has all JS/TS dependencies at their target versions, all quality gates passing, and the dev server healthy.
- Each update batch or individual major jump produces its own commit, so progress is reviewable step by step.

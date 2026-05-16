# Plan: TS/JS Dependency Update

## Solution Approach

Update `package.json` dependencies in two phases: safe minor/patch bumps as a single batch, then one-at-a-time major version jumps in order of increasing risk. After each change, install with `bun install`, run the full quality gate (`just check`, `just lint`, `bun run test:run`), and for majors also verify the dev build (`bun run build` or `vite build`). If any gate fails, revert the commit and write a `blocked.md` note. Each successful change gets its own commit.

## Ordered Steps

### Phase A — Safe Updates (batch)

1. **Update safe minor/patch deps in `package.json`**
   - Files touched: `package.json`
   - Change: bump each safe dep to its target version (in `dependencies` and `devDependencies`)
   - Verification: `bun install` succeeds without warnings

   Target versions:
   - `@tauri-apps/api` → `"^2.11.0"` (currently `"^2"`)
   - `@tauri-apps/plugin-opener` → `"^2.5.4"` (currently `"^2"`)
   - `react` → `"^19.2.6"` (currently `"^19.1.0"`)
   - `react-dom` → `"^19.2.6"` (currently `"^19.1.0"`)
   - `@biomejs/biome` → `"^2.4.15"` (currently `"^2.4.10"`)
   - `@tauri-apps/cli` → `"^2.11.1"` (currently `"^2"`)

2. **Install and verify**
   - Commands:
     ```
     bun install
     just check
     just lint
     bun run test:run
     ```
   - Verification: all commands pass with zero errors
   - If any fail: `git checkout -- package.json bun.lock`, write `blocked.md`

3. **Commit**
   - Commit message: `chore(deps): update safe JS/TS dependencies (minor/patch)`
   - Verify: `git status` clean, commit exists

### Phase B — Major: `@vitejs/plugin-react` 4→6 + `vite` 7→8

4. **Update `@vitejs/plugin-react` and `vite` in `package.json`**
   - Files touched: `package.json`, possibly `vite.config.ts`, `vitest.config.ts`
   - Change: bump `@vitejs/plugin-react` from `"^4.6.0"` to `"^6.0.2"`, `vite` from `"^7.0.4"` to `"^8.0.13"`
   - Check Vite 8 migration notes — `defineConfig(async () => ({...}))` pattern may need changes, `server` options may differ

5. **Install, fix and verify**
   - Commands:
     ```
     bun install
     bun run build    # verify Vite build works (CI-safe, doesn't need Tauri)
     just check
     just lint
     bun run test:run
     ```
   - Check Vite 8 API changes: `clearScreen`, `server.port`, `server.strictPort`, `server.hmr` — these are Tauri-specific so verify they still work
   - If type/build errors: fix config first, then re-run gates. If unfixable, revert.

6. **Commit**
   - Commit message: `chore(deps): update @vitejs/plugin-react to 6.x and vite to 8.x`

### Phase C — Major: `vitest` 3→4 + `@vitest/coverage-v8` 3→4

7. **Update `vitest` and `@vitest/coverage-v8` in `package.json`**
   - Files touched: `package.json`, possibly `vitest.config.ts`
   - Change: bump both from `"^3.2.3"`/`"^3.2.4"` to `"^4.1.6"`/`"^4.1.6"`
   - Check Vitest 4 migration — `globals`, `environment`, `setupFiles` fields may have changed

8. **Install, fix and verify**
   - Commands:
     ```
     bun install
     bun run test:run
     just lint
     ```
   - If test failures: check Vitest 4 API changes, fix config, re-run
   - If unfixable: revert

9. **Commit**
   - Commit message: `chore(deps): update vitest to 4.x and @vitest/coverage-v8 to 4.x`

### Phase D — Major: `jsdom` 26→29

10. **Update `jsdom` in `package.json`**
    - Files touched: `package.json`
    - Change: bump from `"^26.1.0"` to `"^29.1.1"`
    - jsdom 29 is a test-only dependency, low risk for runtime

11. **Install and verify**
    - Commands:
      ```
      bun install
      bun run test:run    # verify jsdom environment still works
      just lint
      ```
    - If test failures: check jsdom 27/28/29 changelogs for DOM API behavior changes

12. **Commit**
    - Commit message: `chore(deps): update jsdom to 29.x`

### Phase E — Major: `typescript` 5→6

13. **Update `typescript` in `package.json`**
    - Files touched: `package.json`, possibly `tsconfig.json`, `src/**/*.ts`, `src/**/*.tsx`
    - Change: bump from `"~5.8.3"` to `"~6.0.3"`
    - **Highest risk step** — TypeScript 6 introduces stricter checking that may produce many type errors

14. **Install and fix type errors**
    - Commands:
      ```
      bun install
      bun run typecheck
      ```
    - Fix type errors iteratively (likely `strict` mode changes, `noUnusedLocals`/`noUnusedParameters` changes)
    - After fixes: `just lint`, `bun run test:run`, `bun run build`

15. **Commit**
    - Commit message: `chore(deps): update typescript to 6.x`

### Phase F — Final checks

16. **Full validation pass**
    - Commands:
      ```
      just check
      just lint
      just fmt
      bun run test:run
      bun run build
      ```
    - Verify everything is green end-to-end

17. **Final commit if any fmt changes**
    - Commit message: `chore: post-update formatting adjustments`

## Risks or Open Questions

- **Vite 8 → Tauri compatibility**: The `server` config in `vite.config.ts` is Tauri-specific (fixed port, HMR, watch ignores). Vite 8 may have changed these options. If `cargo tauri dev` breaks, treat as a blocker.
- **TypeScript 6 strictness**: `noUnusedLocals` and `noUnusedParameters` are already enabled in `tsconfig.json`. TS 6 may add new strict checks. Be prepared to fix type errors across `src/`.
- **Mock compatibility**: Tests use `vi.mock("@tauri-apps/api/core")` and `vi.mock("@tauri-apps/api/event")`. If `@tauri-apps/api` 2.11.x changes exports, test mocks may need updating.
- **Biome schema**: `biome.json` references `https://biomejs.dev/schemas/2.4.10/schema.json`. After updating to 2.4.15, this URL should be updated to match.

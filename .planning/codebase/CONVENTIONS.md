# Coding Conventions

**Analysis Date:** 2026-05-06

Project: **EchoNote** — Tauri v2 + React 19 (TypeScript) + Rust (Edition 2021).

## Naming Patterns

**Files:**
- React components: `PascalCase.tsx` co-located in `src/components/` (e.g. `src/components/RecordView.tsx`, `src/components/ErrorBoundary.tsx`).
- Component tests: `PascalCase.test.tsx` under `src/components/__tests__/` (e.g. `src/components/__tests__/RecordView.test.tsx`).
- Rust modules: `snake_case` directories with `mod.rs` (e.g. `src-tauri/src/commands/meetings.rs`, `src-tauri/src/db/mod.rs`, `src-tauri/src/system_audio/mod.rs`).
- SQL migrations: `NNNN_snake_case.sql` (e.g. `src-tauri/migrations/0001_initial_schema.sql`).

**Functions:**
- TS/React: `camelCase` for functions, `PascalCase` for components/hooks-as-components (`function App()` in `src/App.tsx`, `export function RecordView(...)` in `src/components/RecordView.tsx`).
- Rust: `snake_case` for functions; **all Tauri commands end with the `_command` suffix** (`create_meeting_command`, `start_recording_command`, `transcribe_audio_command` — see registration in `src-tauri/src/lib.rs`).
- Rust internal helpers (non-command) use plain verbs: `create_meeting`, `list_meetings`, `init_db` in `src-tauri/src/db/mod.rs`.

**Variables:**
- TS: `camelCase` (`recordingState`, `selectedMeetingId` in `src/App.tsx`).
- Rust: `snake_case` (`device_id`, `system_device_name` in `src-tauri/src/commands/audio.rs`).
- Rust constants: `SCREAMING_SNAKE_CASE` (`DEFAULT_AUDIO_DEVICE`, `DEFAULT_WHISPER_MODEL_SIZE` in `src-tauri/src/db/mod.rs`; `DEFAULT_OLLAMA_URL`, `DEFAULT_SUMMARY_MODEL` in `src-tauri/src/llm/mod.rs`).

**Types:**
- TS interfaces/types: `PascalCase` (`ApiResponse<T>`, `RecordingResponse`, `RecordViewProps` in `src/components/RecordView.tsx`).
- Rust structs/enums: `PascalCase` (`AppStateExt`, `ApiResponse<T>` in `src-tauri/src/lib.rs`; `Meeting`, `CreateMeetingInput` in `src-tauri/src/db/mod.rs`).
- Request/response DTOs: `XxxRequest` for inbound, `XxxResponse` for outbound (`CreateMeetingRequest` / `MeetingResponse` in `src-tauri/src/commands/meetings.rs`).

## Code Style

**Formatting:**
- Frontend: **Biome 2.4.10** (`biome.json`).
  - Indent: **tabs**.
  - Line width: **100**.
  - JS/TS: double quotes, **semicolons always**, **trailing commas: all**.
  - Excludes: `node_modules`, `dist`, `target`, `.git`, `.vscode`, `.claude`, `src-tauri/gen`, `*.lock`.
- Rust: **`cargo fmt`** with default rustfmt settings (no `rustfmt.toml` present in `src-tauri/`).

**Linting:**
- Biome: `recommended` rules + `correctness.noUnusedVariables: "error"` (`biome.json`).
- TypeScript: `strict: true`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch` (`tsconfig.json`).
- Rust: `cargo clippy -- -D warnings` (warnings are errors — see `lint-rs` recipe in `justfile` and `cargo-clippy` hook in `prek.toml`).

## Import Organization

**Order:** Biome's default import sorting is applied via `biome check --write`. Observed pattern (see `src/components/RecordView.tsx`, `src/App.tsx`):
1. External packages (`@tauri-apps/api/core`, `@tauri-apps/api/event`, `react`, `@testing-library/*`).
2. Local relative imports (`./App.css`, `./components/RecordView`, `../RecordView`).

Rust (e.g. `src-tauri/src/commands/meetings.rs`):
1. `crate::` re-exports of shared types (`use crate::{ApiResponse, AppStateExt};`).
2. Sibling module re-exports (`use db::{create_meeting, ...};`).
3. External crates (`use serde::{Deserialize, Serialize};`, `use tauri::State;`).
4. `use crate::db;` last for module aliasing.

**Path Aliases:**
- None configured. `tsconfig.json` uses `moduleResolution: "bundler"` with relative imports only; no `paths` mapping.

## Error Handling

**Patterns:**
- **Rust internal layer** uses `anyhow::Result<T>` with `.context("...")` for added context (`src-tauri/src/db/mod.rs`: `init_db` uses `.context("Failed to get app data directory")`; `src-tauri/src/llm/mod.rs` imports `use anyhow::{Context, Result};` and returns `Result<SummaryResult>`, raising errors with `anyhow::anyhow!(...)`).
- **Tauri command boundary** translates `anyhow::Error` / `std::io::Error` into a user-friendly `String` via `.map_err(|e| format!("Failed to ...: {}", e))?` (canonical example: `src-tauri/src/commands/meetings.rs::create_meeting_command`, `src-tauri/src/commands/audio.rs::start_recording_command`).
- **Every Tauri command returns `Result<ApiResponse<T>, String>`** (defined in `src-tauri/src/lib.rs`):
  ```rust
  #[derive(Serialize)]
  pub struct ApiResponse<T> {
      pub success: bool,
      pub data: Option<T>,
      pub error: Option<String>,
  }
  ```
  Use `ApiResponse::success(data)` for happy paths and `ApiResponse::error(msg)` for *expected* domain errors (e.g. "not found"); reserve the outer `Err(String)` for *unexpected* infrastructure failures. See `src-tauri/src/commands/meetings.rs::get_meeting_command` for both branches in one function.
- **`thiserror` is NOT used** — only `anyhow` is in `src-tauri/Cargo.toml`.
- **Frontend** mirrors the wrapper with a TS interface `ApiResponse<T> { success; data: T | null; error: string | null }` (declared per-file, e.g. `src/components/RecordView.tsx`). Render-time errors are caught by `src/components/ErrorBoundary.tsx`, which wraps each top-level view in `src/App.tsx`.

## Logging

**Framework:** Rust uses the `log` crate (declared in `src-tauri/Cargo.toml`); frontend uses `console.*`.

**Patterns:**
- Rust: `log::info!`, `log::warn!`, `log::error!` with structured context (`src-tauri/src/commands/meetings.rs::get_meeting_command` logs both the lookup attempt and the DB error before mapping it).
- Always log *before* converting an error to `String` so the original error is preserved in the app log (pattern in `get_meeting_command`).
- Frontend: `console.log` for navigation breadcrumbs (`src/App.tsx`), `console.error` for unexpected failures (`src/components/ErrorBoundary.tsx::componentDidCatch`).

## Comments

**When to Comment:**
- Doc-comment every public Rust struct, function, and constant with `///` (see all definitions in `src-tauri/src/db/mod.rs`: `/// Meeting record from the database`, `/// Initialize the database pool ...`).
- TSDoc `/** ... */` on exported components/props that need explanation (`src/components/ErrorBoundary.tsx`: `/** Optional label shown in the fallback ... */`).
- Inline `//` comments only for non-obvious intent (e.g. the BlackHole / `Send`/`Sync` notes in `AGENTS.md` are reflected in code comments where relevant).

**JSDoc/TSDoc:**
- Used sparingly, primarily for prop documentation on shared components. Most internal hooks/utilities are self-documenting via names and types.

## Function Design

**Size:** No hard limit; Tauri command functions stay short (≤ ~40 lines) by delegating to `db::*` / `audio::*` helpers (compare `create_meeting_command` in `src-tauri/src/commands/meetings.rs` with `create_meeting` in `src-tauri/src/db/mod.rs`).

**Parameters:**
- Tauri commands receive `state: State<'_, AppStateExt>` first, then a single typed request struct or primitive args (`src-tauri/src/commands/meetings.rs::create_meeting_command(state, request: CreateMeetingRequest)`).
- DB helpers take `pool: &Pool<Sqlite>` first, then a typed `CreateXxxInput` (`src-tauri/src/db/mod.rs::create_meeting`).

**Return Values:**
- Tauri commands: `Result<ApiResponse<T>, String>` — never `panic!`, never bare `Result<T, E>`.
- DB / domain helpers: `anyhow::Result<T>` with `Option<T>` for "may not exist" reads (`get_meeting -> Result<Option<Meeting>>`).
- React components: explicit return types are inferred; props use `interface XxxProps`.

## Module Design

**Exports:**
- Rust: each subsystem is one folder with `mod.rs` exposing its public surface (`src-tauri/src/audio/mod.rs`, `src-tauri/src/whisper/mod.rs`). The crate root `src-tauri/src/lib.rs` declares modules and `pub use` shared types (`AppStateExt`, `ApiResponse`).
- Tauri commands are grouped by domain under `src-tauri/src/commands/` (`audio.rs`, `meetings.rs`, `settings.rs`, ...) and re-exported via `src-tauri/src/commands/mod.rs`. They are registered in `tauri::generate_handler![...]` inside `src-tauri/src/lib.rs::run()` — **adding a new command requires editing both the module file and the handler list.**
- React: named exports only (`export function RecordView`, `export class ErrorBoundary`). `src/App.tsx` is the single `default` export.

**Barrel Files:**
- Not used on the frontend — components are imported directly (`import { RecordView } from "./components/RecordView"`).
- Rust uses `commands/mod.rs` as a thin re-export hub (`pub mod audio; pub mod meetings; ...`); the consumer `lib.rs` imports with `use commands::{audio::*, llm::*, ...}`.

## Tauri Command Pattern (Canonical)

Defined in `AGENTS.md` and applied uniformly across `src-tauri/src/commands/*.rs`:

```rust
#[tauri::command]
pub async fn create_meeting_command(
    state: State<'_, AppStateExt>,
    request: CreateMeetingRequest,
) -> Result<ApiResponse<MeetingResponse>, String> {
    let id = create_meeting(&state.db, input)
        .await
        .map_err(|e| format!("Failed to create meeting: {}", e))?;
    Ok(ApiResponse::success(meeting.into()))
}
```

Conventions enforced:
1. `async fn`, `pub`, ends in `_command`.
2. First arg is `State<'_, AppStateExt>`; second arg (if any) is a single `XxxRequest` deserialized struct.
3. Returns `Result<ApiResponse<T>, String>`.
4. All fallible calls use `.map_err(|e| format!("Failed to ...: {}", e))?`.
5. Convert internal domain types to a `XxxResponse` DTO via `From<Domain> for Response` impls (`impl From<db::Meeting> for MeetingResponse` in `src-tauri/src/commands/meetings.rs`).

## SQLx Usage

- Pool type: `sqlx::Pool<sqlx::Sqlite>` stored on `AppStateExt.db` (`src-tauri/src/lib.rs`).
- **Runtime queries** (`sqlx::query` / `sqlx::query_as::<_, T>`) with `.bind(...)` are used throughout `src-tauri/src/db/mod.rs` — **the compile-time `sqlx::query!` macro mentioned in `AGENTS.md` is not currently in use** (no `sqlx-cli` or `DATABASE_URL` setup, no `.sqlx` offline cache).
- Row mapping via `#[derive(sqlx::FromRow)]` on domain structs (`Meeting`, `Transcript`, `Summary`, `Setting` in `src-tauri/src/db/mod.rs`).
- Migrations live in `src-tauri/migrations/` and are run on startup with `sqlx::migrate!("./migrations").run(&pool).await` inside `init_db` (`src-tauri/src/db/mod.rs`).
- Pool config: `max_connections(5)`, `create_if_missing(true)`, `foreign_keys(true)` — set in `init_db`.

## Quality Gates

Defined in `justfile` and enforced pre-commit via `prek.toml`:

| Gate | Command | What it runs |
|---|---|---|
| Typecheck | `just check` | `bun run typecheck` (TS) **and** `cargo check --manifest-path src-tauri/Cargo.toml` (Rust). |
| Lint | `just lint` | `npx @biomejs/biome check .` **and** `cargo clippy -- -D warnings`. |
| Format | `just fmt` | `npx @biomejs/biome check --write .` **and** `cargo fmt`. |
| Tests | `just test-rs [args]` | `cargo test --manifest-path src-tauri/Cargo.toml` (frontend tests run via `bun run test` / `bun run test:run` per `package.json`). |
| Pre-commit | `just pre-commit` | `prek run --all-files` — runs trailing-whitespace, end-of-file-fixer, large-file check, biome-check, typecheck, cargo-fmt, cargo-clippy. |

All four (`check`, `lint`, `fmt`, tests) must pass before commit per `AGENTS.md` § Quality Gates.

---

*Convention analysis: 2026-05-06*

# Codebase Structure

**Analysis Date:** 2026-05-06

## Directory Layout

```
echo-note/
├── src/                              # React 19 + TypeScript frontend
│   ├── main.tsx                      # ReactDOM bootstrap into #root
│   ├── App.tsx                       # Top-level shell + view routing
│   ├── App.css                       # All app styles (single sheet)
│   ├── vite-env.d.ts                 # Vite env types
│   ├── assets/                       # Static frontend assets (e.g. react.svg)
│   ├── components/                   # Feature views
│   │   ├── RecordView.tsx            # Recording → transcription → summary flow
│   │   ├── HistoryView.tsx           # List of saved meetings
│   │   ├── MeetingDetailView.tsx     # Single meeting (audio, transcript, summary)
│   │   ├── SettingsView.tsx          # Audio device, Whisper model, LLM provider
│   │   ├── ErrorBoundary.tsx         # Per-section React error boundary
│   │   └── __tests__/                # Vitest + RTL component tests
│   └── test/setup.ts                 # Mocks @tauri-apps/api for vitest
│
├── src-tauri/                        # Rust backend (Tauri v2)
│   ├── Cargo.toml                    # Crate name `echo_note_lib` (lib + bin)
│   ├── build.rs                      # Runs `tauri_build::build()`
│   ├── tauri.conf.json               # Window, CSP, bundle config
│   ├── capabilities/default.json     # Tauri ACL for the main window
│   ├── migrations/                   # SQLx migrations
│   │   └── 0001_initial_schema.sql   # meetings / transcripts / summaries / settings
│   ├── gen/schemas/                  # AUTO-GENERATED ACL & capability schemas
│   ├── icons/                        # App icons (.png/.icns/.ico)
│   ├── src/
│   │   ├── main.rs                   # Binary entry → echo_note_lib::run()
│   │   ├── lib.rs                    # tauri::Builder, AppStateExt, command registry
│   │   ├── audio/mod.rs              # AudioRecorder, cpal threads, WAV mixing
│   │   ├── system_audio/mod.rs       # BlackHole detection / install
│   │   ├── whisper/mod.rs            # Model registry, download, transcription
│   │   ├── llm/mod.rs                # Ollama + OpenAI-compatible summarization
│   │   ├── db/mod.rs                 # SQLx pool init + CRUD per table
│   │   └── commands/                 # Tauri command handlers (IPC layer)
│   │       ├── mod.rs                # `pub mod` re-exports
│   │       ├── audio.rs              # start/stop recording, list devices, BlackHole
│   │       ├── transcription.rs      # Whisper model + transcribe commands
│   │       ├── llm.rs                # Summary generation, Ollama status
│   │       ├── meetings.rs           # CRUD on meetings
│   │       ├── transcripts.rs        # CRUD on transcripts
│   │       ├── summaries.rs          # CRUD on summaries
│   │       └── settings.rs           # Get/set/list/delete settings
│   ├── tests/                        # Integration tests (cargo test)
│   │   ├── common/mod.rs
│   │   ├── db_integration.rs
│   │   └── meeting_flow.rs
│   └── target/                       # Cargo build output (gitignored)
│
├── scripts/ralph/                    # Ralph autonomous-agent workflow
│   ├── prd.json                      # Story list
│   ├── progress.txt                  # Learned patterns
│   ├── prompt-opencode.md
│   ├── prompt-pi.md
│   └── ralph.sh
│
├── tasks/                            # PRDs and planning docs (per AGENTS.md)
├── public/                           # Static assets served by Vite
├── dist/                             # Vite build output → consumed by Tauri (frontendDist: ../dist)
├── index.html                        # Vite entry HTML, mounts /src/main.tsx
├── vite.config.ts                    # Vite + React plugin, fixed port 1420
├── vitest.config.ts                  # Vitest config (jsdom)
├── tsconfig.json / tsconfig.node.json
├── biome.json                        # Biome lint/format config
├── package.json                      # bun scripts, react 19, @tauri-apps/api
├── bun.lock
├── justfile                          # `just dev / check / lint / fmt / test-rs`
├── prek.toml                         # Pre-commit hook config
├── renovate.json
├── README.md / LICENSE
├── AGENTS.md / CLAUDE.md             # Repo-level agent guidance
└── .planning/codebase/               # This codemap output
```

## Directory Purposes

**`src/`:**
- Purpose: React 19 + TypeScript frontend.
- Contains: A single-page app with view-based routing kept in `App.tsx` `useState`.
- Key files: `src/main.tsx` (mount), `src/App.tsx` (shell), `src/components/RecordView.tsx` (orchestrates the recording → transcription → summary flow).

**`src/components/`:**
- Purpose: Self-contained feature views; one `*View.tsx` per top-level navigation item plus shared `ErrorBoundary.tsx`.
- Contains: All Tauri `invoke()` calls and `listen()` event subscriptions; styles come from `src/App.css`.
- Key files: `RecordView.tsx`, `HistoryView.tsx`, `MeetingDetailView.tsx`, `SettingsView.tsx`.

**`src/components/__tests__/`:**
- Purpose: Vitest + React Testing Library tests, one per component.
- Key files: `RecordView.test.tsx`, etc. Use mocks from `src/test/setup.ts`.

**`src-tauri/src/`:**
- Purpose: Rust backend split by concern.
- Contains: Bootstrap (`main.rs`, `lib.rs`), service modules (`audio/`, `system_audio/`, `whisper/`, `llm/`, `db/`), and the `commands/` IPC layer.
- Key files: `lib.rs` (command registry + state), `audio/mod.rs` (cpal threading), `db/mod.rs` (SQLx CRUD).

**`src-tauri/src/commands/`:**
- Purpose: Thin IPC layer — every public function is `#[tauri::command]` and ends in `_command`.
- Contains: One file per resource. Each handler returns `Result<ApiResponse<T>, String>`.
- Key files: `audio.rs`, `transcription.rs`, `llm.rs`, `meetings.rs`, `settings.rs`, `summaries.rs`, `transcripts.rs`.

**`src-tauri/migrations/`:**
- Purpose: SQLx file-based migrations applied at startup by `sqlx::migrate!("./migrations")` in `src-tauri/src/db/mod.rs:107`.
- Contains: SQL files numbered `NNNN_*.sql`.

**`src-tauri/gen/`:**
- Purpose: AUTO-GENERATED Tauri capability/ACL/permission schemas. Written by `tauri_build::build()` (`src-tauri/build.rs`).
- Contains: `schemas/acl-manifests.json`, `schemas/capabilities.json`, `schemas/desktop-schema.json`, `schemas/macOS-schema.json`.

**`src-tauri/capabilities/`:**
- Purpose: Hand-written Tauri capability declarations referenced from `tauri.conf.json`.
- Key files: `default.json` (grants `core:default` + `opener:default` to `main`).

**`src-tauri/tests/`:**
- Purpose: Rust integration tests (`cargo test` / `just test-rs`).
- Key files: `db_integration.rs`, `meeting_flow.rs`, shared helpers in `common/mod.rs`.

**`scripts/ralph/`:**
- Purpose: Inputs and prompts for the Ralph autonomous-agent workflow described in `AGENTS.md`.
- Key files: `prd.json`, `progress.txt`, `ralph.sh`.

**`tasks/`:**
- Purpose: PRD and planning docs (referenced by `AGENTS.md`'s "Single story focus per commit").

## Key File Locations

**Entry Points:**
- `src/main.tsx`: Frontend bootstrap (ReactDOM into `#root` from `index.html`).
- `src/App.tsx`: Top-level shell; switches between Record / History / Settings / MeetingDetail.
- `src-tauri/src/main.rs`: Rust binary entry; sets Windows subsystem and calls `echo_note_lib::run()`.
- `src-tauri/src/lib.rs`: Real entry — Tauri builder, plugin registration, DB init, `manage(AppStateExt)`, and the full `tauri::generate_handler![…]` list.

**Configuration:**
- `src-tauri/tauri.conf.json`: Window (800×600), CSP (`localhost:*`, `api.openai.com`), bundle, `devUrl=http://localhost:1420`, runs `bun run dev`.
- `src-tauri/Cargo.toml`: `lib name = "echo_note_lib"`; deps `tauri 2`, `sqlx 0.8` (sqlite + chrono + migrate), `cpal 0.15`, `hound 3.5`, `whisper-rs 0.13`, `reqwest 0.12`, `tokio full`.
- `vite.config.ts`: Fixed port 1420, ignores `**/src-tauri/**`.
- `vitest.config.ts` + `src/test/setup.ts`: jsdom + Tauri API mocks.
- `biome.json` / `prek.toml` / `justfile`: lint, hooks, recipes.
- `package.json`: `bun` as runner; scripts `dev`, `build`, `typecheck`, `lint`, `tauri`, `test`.
- `src-tauri/capabilities/default.json`: ACL for the main window.

**Core Logic:**
- `src-tauri/src/audio/mod.rs`: `AudioRecorder`, per-device `std::thread`, `cpal::Stream` (kept thread-local), WAV mixing/encoding.
- `src-tauri/src/whisper/mod.rs`: Model registry (`WHISPER_MODELS`), HF download streaming, `transcribe_audio` chunked inference.
- `src-tauri/src/llm/mod.rs`: `generate_summary` (Ollama `/api/generate`) and `generate_summary_api` (OpenAI-compatible `/v1/chat/completions`); structured prompt + parser.
- `src-tauri/src/db/mod.rs`: SQLx pool, table structs, CRUD per table, default-settings seeding.
- `src-tauri/src/system_audio/mod.rs`: BlackHole detection (system_profiler JSON, fallback grep, HAL plugin scan), Homebrew/manual install.
- `src/components/RecordView.tsx`: Frontend orchestrator for the full pipeline.

**Testing:**
- `src/components/__tests__/*.test.tsx`: Frontend component tests (Vitest + RTL).
- `src/test/setup.ts`: Mocks `@tauri-apps/api/core` and `@tauri-apps/api/event`.
- `src-tauri/tests/`: Rust integration tests.
- Inline `#[cfg(test)] mod tests` blocks in `whisper/mod.rs`, `llm/mod.rs`, `system_audio/mod.rs` cover pure helpers.

## Naming Conventions

**Files:**
- React components: PascalCase, ending in `View.tsx` for navigable screens (`RecordView.tsx`, `HistoryView.tsx`).
- Component tests: `<Name>.test.tsx` colocated under `__tests__/`.
- Rust modules: `snake_case`, each module is a directory with `mod.rs` (e.g. `audio/mod.rs`, `system_audio/mod.rs`).
- SQL migrations: `NNNN_<description>.sql` (e.g. `0001_initial_schema.sql`).

**Symbols:**
- Tauri commands: `<verb>_<resource>_command` (e.g. `start_recording_command`, `transcribe_audio_command`, `check_ollama_status_command`).
- Response DTOs: `<Thing>Response` (e.g. `MeetingResponse`, `RecordingResponse`, `WhisperModelInfoResponse`).
- Input DTOs: `Create<Thing>Request` for command input, `Create<Thing>Input` for the DB layer (e.g. `CreateMeetingRequest` → `CreateMeetingInput`).
- Default constants: `DEFAULT_<KEY>` in `src-tauri/src/db/mod.rs` and per-module (`DEFAULT_OLLAMA_URL`, `DEFAULT_SUMMARY_MODEL`).
- Tauri events: kebab-case strings (`whisper-download-progress`, `transcription-progress`).
- Settings keys: snake_case strings (`audio_device`, `whisper_model_size`, `llm_provider`, `api_key`, `api_endpoint`, `api_model`).

**Directories:**
- Frontend feature code grouped under `src/components/` (flat).
- Backend code grouped by *concern* (`audio/`, `whisper/`, `llm/`, `db/`, `commands/`), not by layer.

## Where to Add New Code

**New Tauri command:**
- Implementation: pick or add a file under `src-tauri/src/commands/` and write a `pub async fn <name>_command(...)` returning `Result<ApiResponse<T>, String>`.
- Registration: add the function to the `tauri::generate_handler![…]` macro in `src-tauri/src/lib.rs:71`.
- Re-export: ensure the file is listed in `src-tauri/src/commands/mod.rs`.
- Frontend: call via `invoke<ApiResponse<T>>("<name>", { … })` from a component.

**New domain logic / service:**
- Implementation: add a new module under `src-tauri/src/<concern>/mod.rs` (or extend an existing one). Keep it independent of `tauri::State` — accept primitives and the SQLx pool / `AppHandle` only when needed.
- Wiring: declare the module in `src-tauri/src/lib.rs:1`-`:6` and call it from a command.

**New table or schema change:**
- Migration: add `src-tauri/migrations/NNNN_<description>.sql` (next number after `0001_initial_schema.sql`). It runs automatically at startup via `sqlx::migrate!`.
- Code: add struct + CRUD functions in `src-tauri/src/db/mod.rs`, then expose commands under `src-tauri/src/commands/`.

**New frontend view:**
- Component: `src/components/<Name>View.tsx`.
- Routing: extend the `View` union and `switch` in `src/App.tsx:9`-`:84` and add a `<button>` in the sidebar nav.
- Tests: add `src/components/__tests__/<Name>View.test.tsx`.

**New setting:**
- Add a `DEFAULT_<KEY>` constant in `src-tauri/src/db/mod.rs:80`.
- Seed it in `init_default_settings` (`src-tauri/src/db/mod.rs:441`).
- Map the key in `get_setting_command` (`src-tauri/src/commands/settings.rs:46`).
- Surface it in `src/components/SettingsView.tsx`.

**Shared utilities:**
- Frontend: currently no `lib/` — keep helpers colocated in the component, or add `src/lib/` if they grow. Type aliases for IPC payloads are duplicated per file today.
- Rust: helpers inside the relevant `<concern>/mod.rs`, kept private unless reused.

## Special Directories

**`src-tauri/gen/`:**
- Purpose: Tauri-generated permission/capability/ACL schemas referenced via `$schema` in `src-tauri/capabilities/default.json`.
- Generated: Yes — by `tauri_build::build()` in `src-tauri/build.rs`.
- Committed: Yes (so the schemas are available without a build step). **Do not hand-edit.**

**`src-tauri/target/`:**
- Purpose: Cargo build artifacts.
- Generated: Yes. Committed: No (gitignored).

**`dist/`:**
- Purpose: Vite production build, consumed by Tauri (`tauri.conf.json` `frontendDist: "../dist"`).
- Generated: Yes by `bun run build`. Committed: No.

**`node_modules/` / `bun.lock`:**
- `node_modules/` generated by `bun install`; only `bun.lock` is committed.

**`scripts/ralph/`:**
- Purpose: Autonomous Ralph agent inputs (PRD, progress journal, prompts, runner).
- Generated: No — hand-maintained.
- Committed: Yes.

**Runtime data (NOT in repo):**
- SQLite DB at `<app_data_dir>/echo_note.db` (`src-tauri/src/db/mod.rs:96`).
- WAV recordings at `<app_data_dir>/recordings/` (`src-tauri/src/audio/mod.rs:585`).
- Whisper models at `<app_data_dir>/models/` (`src-tauri/src/whisper/mod.rs:53`). On macOS this is `~/Library/Application Support/com.huynhdung.echo-note/`.

---

*Structure analysis: 2026-05-06*

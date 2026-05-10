# Architecture

**Analysis Date:** 2026-05-06

## Pattern Overview

**Overall:** Tauri v2 desktop app — React (TypeScript) frontend + Rust backend talking over Tauri's IPC. Inside the Rust backend the code follows a thin **command → service module → SQLx repository** layering, with a dedicated thread-per-stream **audio actor** for `cpal` (which is `!Send`/`!Sync`).

**Key Characteristics:**
- Privacy-first / fully local by default: SQLite on disk (`src-tauri/src/db/mod.rs:88`), Whisper inference in-process via `whisper-rs` (`src-tauri/src/whisper/mod.rs:235`), Ollama on `http://localhost:11434` (`src-tauri/src/llm/mod.rs:6`). Cloud (OpenAI-compatible) is opt-in.
- One unified Rust app state (`AppStateExt`) registered with Tauri's `Manager` and accessed in commands as `State<'_, AppStateExt>` (`src-tauri/src/lib.rs:13`, `src-tauri/src/lib.rs:67`).
- Strict frontend/backend boundary: every backend call goes through `invoke("…_command", …)` and returns an `ApiResponse<T>` envelope (`src-tauri/src/lib.rs:21`, `src/components/RecordView.tsx:6`).
- Heavy work (Whisper transcription, mic test) is offloaded with `tokio::task::spawn_blocking` so the async runtime stays responsive (`src-tauri/src/commands/transcription.rs:55`, `src-tauri/src/commands/audio.rs:107`).
- Streamed progress events use the Tauri event bus (`whisper-download-progress`, `transcription-progress`) instead of polling (`src-tauri/src/whisper/mod.rs:155`, `src/components/RecordView.tsx:89`).

## Layers

**Frontend View Layer (React 19 + TS):**
- Purpose: UI, local view state, IPC orchestration.
- Location: `src/`
- Contains: Top-level shell `src/App.tsx`, four feature views in `src/components/` (`RecordView.tsx`, `HistoryView.tsx`, `MeetingDetailView.tsx`, `SettingsView.tsx`), plus `ErrorBoundary.tsx`.
- Depends on: `@tauri-apps/api/core` (`invoke`) and `@tauri-apps/api/event` (`listen`).
- Used by: Bootstrapped from `src/main.tsx` into `<div id="root">` from `index.html`.

**Tauri Command Layer (Rust):**
- Purpose: IPC surface — validates input, fetches settings, calls service modules, persists results, wraps everything in `ApiResponse<T>`.
- Location: `src-tauri/src/commands/`
- Contains: One file per resource (`audio.rs`, `transcription.rs`, `llm.rs`, `meetings.rs`, `transcripts.rs`, `summaries.rs`, `settings.rs`) re-exported from `commands/mod.rs`. All handlers are registered in `tauri::generate_handler![…]` in `src-tauri/src/lib.rs:71`.
- Depends on: `db`, `audio`, `whisper`, `llm`, `system_audio` modules; `AppStateExt`.
- Used by: Tauri runtime → frontend `invoke()` calls.

**Service / Domain Layer (Rust):**
- Purpose: Pure-ish domain logic that does not know about Tauri commands.
- Location: `src-tauri/src/audio/mod.rs`, `src-tauri/src/whisper/mod.rs`, `src-tauri/src/llm/mod.rs`, `src-tauri/src/system_audio/mod.rs`.
- Contains:
  - `AudioRecorder` (`src-tauri/src/audio/mod.rs:53`) — manages two recording threads (mic + optional BlackHole system audio) over `mpsc` channels and shared `Arc<Mutex<RecordingState>>`.
  - Whisper download/transcribe pipeline (`src-tauri/src/whisper/mod.rs:101`, `:235`) using `reqwest` streaming, `hound` WAV reader, and `whisper-rs` inference.
  - Ollama + OpenAI-compatible summarization (`src-tauri/src/llm/mod.rs:84`, `:194`) plus prompt builder/parser.
  - BlackHole detection / install via `system_profiler`, Homebrew, manual download (`src-tauri/src/system_audio/mod.rs:11`).
- Depends on: `tauri::AppHandle` (only for `path().app_data_dir()` and event emission), `cpal`, `hound`, `whisper-rs`, `reqwest`.
- Used by: Command layer.

**Data Layer (Rust + SQLx + SQLite):**
- Purpose: Persistence of meetings, transcripts, summaries, settings.
- Location: `src-tauri/src/db/mod.rs`, schema in `src-tauri/migrations/0001_initial_schema.sql`.
- Contains: Strongly-typed structs (`Meeting`, `Transcript`, `Summary`, `Setting`) with `sqlx::FromRow`, plus free CRUD functions per table. `init_db` opens the SQLite pool at `<app_data_dir>/echo_note.db` and runs migrations via `sqlx::migrate!("./migrations")` (`src-tauri/src/db/mod.rs:88`).
- Depends on: `sqlx::Pool<Sqlite>`, `chrono`, `tauri::Manager` (only for the data-dir path).
- Used by: All command handlers via `state.db`.

**Infrastructure / Bootstrap:**
- Location: `src-tauri/src/main.rs` (release entry, `cfg_attr(windows_subsystem = "windows")`) → `src-tauri/src/lib.rs::run()` (`tauri::Builder` setup, plugin registration, DB init, state management, handler registration).

## Data Flow

**End-to-end recording → transcription → summary:**
1. User clicks "Record" in `RecordView`. UI calls `invoke("start_recording_command")` (`src/components/RecordView.tsx:237`).
2. `start_recording_command` reads `audio_device` from settings, checks BlackHole, then calls `AudioRecorder::start_recording` (`src-tauri/src/commands/audio.rs:40`).
3. `AudioRecorder` spawns one `std::thread` per device. Each thread builds a `cpal::Stream` and pushes f32 samples into an `Arc<Mutex<Vec<f32>>>` until a `RecordingCommand::Stop` arrives over its `mpsc` channel (`src-tauri/src/audio/mod.rs:332`, `:419`). The stream is created and dropped inside the same thread because `cpal::Stream` is `!Send`/`!Sync`.
4. User clicks "Stop". `stop_recording_command` joins both threads, mixes mic + system streams (`mix_audio_streams`, `src-tauri/src/audio/mod.rs:295`), encodes to 16-bit PCM mono WAV with `hound` into `<app_data_dir>/recordings/recording_<timestamp>.wav` (`src-tauri/src/audio/mod.rs:240`).
5. UI prompts for a title and calls `create_meeting_command`, which writes a `meetings` row (`src-tauri/src/commands/meetings.rs:43`).
6. UI immediately calls `transcribe_audio_command` with the meeting id + audio path. The command reads `whisper_model_size` from settings and runs `whisper::transcribe_audio` inside `spawn_blocking` (`src-tauri/src/commands/transcription.rs:48`). Progress is emitted as `transcription-progress` events while audio is processed in 30 s × 16 kHz chunks (`src-tauri/src/whisper/mod.rs:262`, `:355`). The transcript text is persisted via `create_transcript`.
7. UI calls `generate_summary_command` with the transcript. Based on `llm_provider` setting, it either POSTs to Ollama `/api/generate` or to an OpenAI-compatible `/v1/chat/completions` (URL normalized in `normalize_api_endpoint`, `src-tauri/src/commands/llm.rs:18`). The structured text response is parsed into `key_points` / `decisions` / `action_items` (`src-tauri/src/llm/mod.rs:288`) and saved with `create_summary`.
8. UI navigates to `MeetingDetailView` for the new `meeting_id`.

**Whisper model download:**
1. `SettingsView` calls `download_whisper_model_command`.
2. Backend streams from `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/<file>` and emits `whisper-download-progress` events every ~5 % (`src-tauri/src/whisper/mod.rs:140`).

**State Management:**
- Frontend: pure local `useState` per view; the App-level shell in `src/App.tsx:11` keeps `currentView` and `selectedMeetingId`. There is no Redux/Zustand/Context — server state lives in SQLite and is re-fetched per view.
- Backend: a single `AppStateExt { db: Pool<Sqlite>, audio_recorder: Mutex<AudioRecorder> }` registered with `app_handle.manage(...)` (`src-tauri/src/lib.rs:13`, `src-tauri/src/lib.rs:64`). Recording state inside `AudioRecorder` is shared between threads via `Arc<Mutex<RecordingState>>`.
- User settings (audio device, Whisper model size, LLM provider, API key/endpoint/model) live in the SQLite `settings` table; defaults are seeded by `init_default_settings` (`src-tauri/src/db/mod.rs:441`).

## Key Abstractions

**`ApiResponse<T>`:**
- Purpose: Uniform success/error envelope across every Tauri command.
- Examples: defined in `src-tauri/src/lib.rs:21`; mirrored on the frontend as `interface ApiResponse<T>` (`src/components/RecordView.tsx:6`).
- Pattern: Commands return `Result<ApiResponse<T>, String>`; `Result::Err` is for unexpected failures, `ApiResponse::error` is for expected business errors (e.g. "Meeting not found").

**`AppStateExt`:**
- Purpose: Singleton backend state injected into commands via `State<'_, AppStateExt>`.
- Examples: `src-tauri/src/lib.rs:13`, used by every command (e.g. `src-tauri/src/commands/meetings.rs:46`).

**`AudioRecorder` actor:**
- Purpose: Encapsulate the `!Send`/`!Sync` `cpal::Stream` so it never crosses thread boundaries; expose a simple start/stop API to commands.
- Examples: `src-tauri/src/audio/mod.rs:53`. Pattern: dedicated `std::thread` per stream, `mpsc::channel` for stop signal, `Arc<Mutex<Vec<f32>>>` for sample buffer, `AtomicBool` for fast-path stop check inside the cpal data callback.

**Whisper model registry:**
- Purpose: Static table of supported `ggml-*.bin` models with expected sizes/labels.
- Examples: `WHISPER_MODELS` in `src-tauri/src/whisper/mod.rs:8`.

**Settings repository pattern:**
- Purpose: Key/value store with typed default constants (`DEFAULT_AUDIO_DEVICE`, `DEFAULT_WHISPER_MODEL_SIZE`, `DEFAULT_LLM_PROVIDER`, …) at `src-tauri/src/db/mod.rs:80`. Commands look up settings on demand rather than caching.

## Entry Points

**Rust binary:**
- Location: `src-tauri/src/main.rs`
- Triggers: `cargo tauri dev` / `cargo tauri build`.
- Responsibilities: Calls `echo_note_lib::run()`. Sets `windows_subsystem = "windows"` in release.

**Rust library / Tauri builder:**
- Location: `src-tauri/src/lib.rs::run()` (`src-tauri/src/lib.rs:50`).
- Responsibilities: Register `tauri_plugin_opener`, in `setup` open the SQLite pool, seed defaults, build `AppStateExt`, and wire all `*_command` handlers via `tauri::generate_handler!` (`src-tauri/src/lib.rs:71`).

**Frontend bootstrap:**
- Location: `src/main.tsx` mounts `<App />` into `#root` from `index.html`. Vite dev server runs on fixed port `1420` (`vite.config.ts:14`), referenced by `tauri.conf.json`'s `devUrl`.

**Tauri commands (IPC entry points):**
- Location: `src-tauri/src/commands/*.rs`. All names end with `_command` and are listed in `src-tauri/src/lib.rs:71`-`:113`. Frontend calls them via `invoke("<name>", { … })`.

**Build / config entry points:**
- `src-tauri/build.rs` runs `tauri_build::build()` to generate ACL/capability schemas into `src-tauri/gen/schemas/`.
- `src-tauri/tauri.conf.json` defines the window, CSP (allows `localhost:*`, `api.openai.com`), and bundle metadata.
- `src-tauri/capabilities/default.json` grants `core:default` + `opener:default` to the `main` window.

## Error Handling

**Strategy:** Rust uses `anyhow::Result` inside service modules. Command handlers convert errors at the boundary into `Result<ApiResponse<T>, String>` — unrecoverable Tauri-side errors become `Err(String)`, expected domain errors become `Ok(ApiResponse::error(...))`. The frontend always inspects `response.success` / `response.error`.

**Patterns:**
- Mutex poisoning is tolerated with `unwrap_or_else(|e| e.into_inner())` inside `AudioRecorder` so a panic in one cpal callback doesn't kill the whole recorder (`src-tauri/src/audio/mod.rs:65`).
- System-audio (BlackHole) failures are non-fatal — the recorder logs and falls back to mic-only (`src-tauri/src/audio/mod.rs:194`).
- Frontend wraps each top-level view in an `ErrorBoundary` keyed by section name (`src/App.tsx:18`, `src/components/ErrorBoundary.tsx`).
- Vitest setup mocks `@tauri-apps/api/core` and `…/event` so component tests can run in jsdom (`src/test/setup.ts`).

## Cross-Cutting Concerns

**Logging:** The Rust `log` crate is used throughout (`log::info!`, `log::warn!`, `log::error!`); see e.g. `src-tauri/src/whisper/mod.rs:175`, `src-tauri/src/system_audio/mod.rs:18`. No explicit logger is initialized — Tauri's default stdout logger picks it up in dev.

**Validation:** Done at the command boundary: empty `api_key` rejected (`src-tauri/src/commands/llm.rs:88`), invalid Whisper model size rejected (`src-tauri/src/whisper/mod.rs:64`), missing audio device falls back to default (`src-tauri/src/audio/mod.rs:344`). The frontend validates required title input before `create_meeting_command` (`src/components/RecordView.tsx:285`).

**Authentication:** None for the local app. For the optional cloud LLM provider, an API key from settings is sent as `Authorization: Bearer …` (`src-tauri/src/llm/mod.rs:124`).

**Security / sandboxing:** Tauri capability set is minimal (`src-tauri/capabilities/default.json`). CSP restricts `connect-src` to `self`, `localhost:*`, and `api.openai.com` (`src-tauri/tauri.conf.json:24`).

**Generated code:** `src-tauri/gen/schemas/` (capabilities, ACL manifests, desktop/macOS schemas) is produced by `tauri_build::build()` in `src-tauri/build.rs` — do not edit by hand.

---

*Architecture analysis: 2026-05-06*

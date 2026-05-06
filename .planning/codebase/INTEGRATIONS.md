# External Integrations

**Analysis Date:** 2026-05-06

EchoNote is privacy-first and runs almost entirely on the user's machine. External integrations are limited to: optional cloud LLM (OpenAI-compatible), local Ollama, Hugging Face for one-time Whisper model downloads, and the BlackHole virtual audio driver / Homebrew for system-audio capture setup.

## APIs & External Services

**LLM — Local:**
- **Ollama** — local model inference for meeting summarization
  - Default endpoint: `http://localhost:11434` (`src-tauri/src/llm/mod.rs:6` — `DEFAULT_OLLAMA_URL`)
  - Endpoints used:
    - `POST {ollama_url}/api/generate` — non-streaming summary generation (`src-tauri/src/llm/mod.rs:184`)
    - `GET  {ollama_url}/api/tags` — health check via `check_ollama_status` (`src-tauri/src/llm/mod.rs:355`)
  - SDK/Client: `reqwest` `0.12` (`src-tauri/src/llm/mod.rs:107`, `:180`, `:351`)
  - Auth: none (localhost)
  - Provider key: `PROVIDER_OLLAMA = "ollama"` (`src-tauri/src/llm/mod.rs:45`); selected via `llm_provider` setting (default `ollama`, `src-tauri/src/db/mod.rs:80`)

**LLM — Cloud (optional):**
- **OpenAI-compatible Chat Completions API** (default vendor: OpenAI; any compatible endpoint works)
  - Default endpoint: `https://api.openai.com/v1` (`src-tauri/src/db/mod.rs:82` — `DEFAULT_API_ENDPOINT`)
  - Default model: `gpt-4o-mini` (`src-tauri/src/db/mod.rs:83` — `DEFAULT_API_MODEL`)
  - Path: caller posts to `api_endpoint` directly (expected to be the chat-completions URL); request schema is OpenAI-compatible `messages: [{role, content}]` (`src-tauri/src/llm/mod.rs:48-79`, `:113`)
  - SDK/Client: raw `reqwest` calls (no OpenAI SDK)
  - Auth: `Authorization: Bearer {api_key}` header (`src-tauri/src/llm/mod.rs:115`)
  - Provider key: `PROVIDER_API = "api"` (`src-tauri/src/llm/mod.rs:46`); api key stored in `settings.api_key` (default empty, `src-tauri/src/db/mod.rs:81`)
  - CSP allow-list: `connect-src` includes `https://api.openai.com https://*.openai.com` (`src-tauri/tauri.conf.json`)

**Model Downloads:**
- **Hugging Face** (`huggingface.co`) — distributes whisper.cpp `ggml-*.bin` Whisper models
  - URL pattern: `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{filename}` (`src-tauri/src/whisper/mod.rs:119`)
  - Model catalog (10 entries) defined in `WHISPER_MODELS` constant: `tiny`, `tiny-q5_1`, `base`, `base-q5_1`, `small`, `small-q5_1`, `medium`, `medium-q5_0`, `large-v3-turbo`, `large-v3-turbo-q5_0` (`src-tauri/src/whisper/mod.rs:8-37`)
  - SDK/Client: streaming `reqwest::get` with `futures-util` chunk loop; emits `whisper-download-progress` Tauri events to the UI (`src-tauri/src/whisper/mod.rs:127`, `:170`, `:192`)
  - Auth: none (anonymous public download)

**System / OS Integrations:**
- **BlackHole 2ch** — Existential Audio's virtual audio driver, used to route system audio into the recorder so it can be mixed with the mic
  - Driver name constant: `BLACKHOLE_DRIVER_NAME = "BlackHole2ch"`, bundle id `audio.existential.BlackHole2ch` (`src-tauri/src/system_audio/mod.rs:7-9`)
  - Detection: enumerates `cpal` input devices and falls back to listing `/Library/Audio/Plug-Ins/HAL` for any plug-in containing `blackhole` (`src-tauri/src/system_audio/mod.rs:12-73`)
  - Install paths exposed via Tauri commands `check_blackhole_status_command`, `install_blackhole_command`, `install_blackhole_homebrew_command` (`src-tauri/src/lib.rs`):
    - Manual: opens `https://github.com/ExistentialAudio/BlackHole` in the user's browser via `open` (`src-tauri/src/system_audio/mod.rs:171-176`)
    - Homebrew: drives Terminal.app via `osascript` to run `brew reinstall blackhole-2ch` (`src-tauri/src/system_audio/mod.rs:200-211`)
- **Homebrew** (`brew`) — detected via `which brew`; offered as an install method for BlackHole (`src-tauri/src/system_audio/mod.rs:181-196`); install URL surfaced to user: `https://brew.sh`
- **Tauri Opener Plugin** (`tauri-plugin-opener`) — used to open external URLs from the backend (`src-tauri/Cargo.toml`, `src-tauri/capabilities/default.json` permission `opener:default`)

## Data Storage

**Databases:**
- **SQLite** (embedded, single-file)
  - Connection: `sqlite://{app_data_dir}/...` constructed at startup (`src-tauri/src/db/mod.rs:86-96`); path resolved via Tauri's `app_data_dir()` (per `AGENTS.md`: `~/Library/Application Support/echo-note/`)
  - Client: `sqlx 0.8` with `runtime-tokio`, `sqlite`, `chrono`, `migrate` features (`src-tauri/Cargo.toml`)
  - Schema (`src-tauri/migrations/0001_initial_schema.sql`):
    - `meetings(id, title, date, duration_seconds, audio_path, created_at)`
    - `transcripts(id, meeting_id FK→meetings ON DELETE CASCADE, content, created_at)`
    - `summaries(id, meeting_id FK→meetings ON DELETE CASCADE, key_points, decisions, action_items, created_at)`
    - `settings(id, key UNIQUE, value, created_at, updated_at)`
  - Seeded defaults via `init_default_settings` (`src-tauri/src/db/mod.rs:454-481`)
  - Env var: none — connection string is computed from app data dir

**File Storage:**
- **Local filesystem only**
  - Audio recordings: WAV files written via `hound` to paths recorded in `meetings.audio_path` (`src-tauri/src/audio/mod.rs`)
  - Whisper models: `models/` subdirectory of app data dir (`src-tauri/src/whisper/mod.rs`); browse via `open_models_folder_command` (`src-tauri/src/lib.rs`)

**Caching:**
- None. Downloaded Whisper model files act as a manual on-disk cache — `check_whisper_model_command` reports presence, `download_whisper_model_command` fetches if missing.

## Authentication & Identity

**Auth Provider:**
- None for the app itself (single local user, no login)
- Implementation for outbound LLM:
  - Cloud LLM uses bearer token from `settings.api_key` (`src-tauri/src/db/mod.rs:81`, `src-tauri/src/llm/mod.rs:115`)
  - Ollama local API has no auth

## Monitoring & Observability

**Error Tracking:**
- None. No Sentry / Bugsnag / Crashlytics dependency in `package.json` or `src-tauri/Cargo.toml`.

**Logs:**
- Rust: `log` crate (`log = "0.4"` in `src-tauri/Cargo.toml`); used throughout (`src-tauri/src/llm/mod.rs`, `src-tauri/src/whisper/mod.rs`, `src-tauri/src/system_audio/mod.rs`). Top-level errors surfaced via `log::error!` in `src-tauri/src/lib.rs`.
- No remote log shipping; logs go to Tauri's default stderr/console.

## CI/CD & Deployment

**Hosting:**
- Distributed as a desktop bundle. `src-tauri/tauri.conf.json` sets `bundle.targets = "all"` and `identifier = com.huynhdung.echo-note`. No server-side hosting.

**CI Pipeline:**
- No `.github/workflows` directory present in the repo at analysis time.
- Local quality gates enforced via:
  - `prek` pre-commit hooks (`prek.toml`): trailing-whitespace, end-of-file-fixer, large-file check, biome-check, tsc, cargo fmt, cargo clippy
  - `just` recipes (`justfile`): `check`, `lint`, `fmt`, `pre-commit`, `test-rs`
- Dependency upkeep: **Renovate** (`renovate.json`, extends `config:recommended`)

## Environment Configuration

**Required env vars:**
- None required for the app to launch.
- Optional/dev:
  - `TAURI_DEV_HOST` — LAN HMR host for `vite dev` (`vite.config.ts`)
  - `justfile` has `set dotenv-load`, so any local `.env` will be sourced for `just` recipes only

**User-facing config (stored in SQLite `settings` table, not env):**
- `audio_device` (default `default`)
- `whisper_model_size` (default `small`)
- `llm_provider` — `ollama` (default) or `api`
- `api_endpoint` (default `https://api.openai.com/v1`)
- `api_model` (default `gpt-4o-mini`)
- `api_key` (default empty — user-supplied for cloud LLM)
- Defined in `src-tauri/src/db/mod.rs:78-83`, seeded by `init_default_settings` (`src-tauri/src/db/mod.rs:454-481`)

**Secrets location:**
- API key is stored in plaintext in the local SQLite `settings` table (`src-tauri/migrations/0001_initial_schema.sql`, key `api_key`). No OS keychain integration is present.

## Webhooks & Callbacks

**Incoming:**
- None. The app exposes no HTTP server. All inbound interaction is via Tauri IPC commands registered in `src-tauri/src/lib.rs` (`invoke_handler!`), e.g. meetings/transcripts/summaries/settings CRUD, `start_recording_command`, `transcribe_audio_command`, `generate_summary_command`, `check_ollama_status_command`, `check_blackhole_status_command`, `download_whisper_model_command`, etc.

**Outgoing:**
- HTTP requests only (no webhooks):
  - Ollama: `POST /api/generate`, `GET /api/tags` (`src-tauri/src/llm/mod.rs:184`, `:355`)
  - OpenAI-compatible chat completions: `POST {api_endpoint}` with bearer auth (`src-tauri/src/llm/mod.rs:113`)
  - Hugging Face: `GET https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{filename}` (`src-tauri/src/whisper/mod.rs:120`)

**Internal events (Tauri → UI):**
- `whisper-download-progress` — emitted during model download (`src-tauri/src/whisper/mod.rs:170`, `:192`)
- Additional `app_handle.emit(...)` calls during transcription pipeline (`src-tauri/src/whisper/mod.rs:276`, `:298`, `:366`, `:385`, `:417`)

---

*Integration audit: 2026-05-06*

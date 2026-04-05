# External Integrations

**Analysis Date:** 2026-04-06

## APIs & External Services

**Local LLM:**
- Ollama API - Local LLM for meeting summaries
- Endpoint: `http://localhost:11434` (default)
- SDK/Client: `reqwest` (HTTP) - planned for summary generation
- Auth: None required for local Ollama
- Fallback: OpenAI API if configured (via settings `api_key`, `api_endpoint`)

**Model Distribution:**
- Hugging Face - Whisper model downloads
- URL: `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{filename}`
- Models: tiny (~39MB), base (~74MB), small (~244MB), medium (~769MB)
- Client: `reqwest` with streaming (`futures-util`)

## Data Storage

**Databases:**
- SQLite via SQLx
- Location: `~/Library/Application Support/echo-note/echo_note.db`
- Connection: Pool-based (max 5 connections)
- Schema: 4 tables (`meetings`, `transcripts`, `summaries`, `settings`)
- Client: `sqlx` with compile-time query checking

**File Storage:**
- Local filesystem only
- Recordings: `~/Library/Application Support/echo-note/recordings/`
- Whisper models: `~/Library/Application Support/echo-note/models/`
- No cloud storage integration (privacy-first design)

**Caching:**
- None - Whisper models stored locally but not cached in memory
- Audio data buffered in memory during recording only

## Authentication & Identity

**Auth Provider:**
- None - Single-user local desktop app
- No login required
- API keys for external LLM providers stored locally in SQLite

## Monitoring & Observability

**Error Tracking:**
- None - Errors logged to console only via `log` crate

**Logs:**
- Rust: `log` crate with `eprintln!` for stream errors
- Tauri: Built-in logging via webview console
- No structured logging or external log aggregation

## CI/CD & Deployment

**Hosting:**
- GitHub (source code)
- Tauri Cloud (optional, for updates)

**CI Pipeline:**
- None configured currently
- Manual quality gates via `just check`, `just lint`, `just pre-commit`

## Environment Configuration

**Required env vars:**
- None at runtime
- All configuration via SQLite settings table

**Secrets location:**
- API keys stored in SQLite `settings` table (unencrypted currently)
- Default: Empty API key and endpoint

**Default Settings:**
- `audio_device`: `"default"`
- `whisper_model_size`: `"small"`
- `llm_provider`: `"ollama"`
- `api_key`: `""`
- `api_endpoint`: `"https://api.openai.com/v1"`

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None

## System Integration

**Audio System:**
- BlackHole2ch virtual audio driver (macOS)
- CoreAudio via `system_profiler SPAudioDataType -json`
- cpal for cross-platform audio capture

**File System:**
- Tauri filesystem APIs via `tauri::Manager`
- `app_data_dir()` for app-specific storage
- `resource_dir()` for bundled resources (BlackHole installer)

---

*Integration audit: 2026-04-06*

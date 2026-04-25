# External Integrations

**Analysis Date:** 2026-04-25

## APIs & External Services

**LLM/Summarization:**
- Ollama (localhost:11434) - Local LLM inference for meeting summaries
- OpenAI-compatible API - Fallback for cloud-based LLM (configurable endpoint)
- SDK/Client: reqwest (Rust HTTP client)
- Auth: api_key setting in database (for cloud providers)

**Speech-to-Text:**
- OpenAI Whisper - Local transcription via whisper-rs
- Model download source: HuggingFace (ggerganov/whisper.cpp)
- SDK/Client: whisper-rs 0.13 (Rust bindings to whisper.cpp)

**System Integration:**
- BlackHole2ch - Virtual audio driver for macOS system audio capture
- Homebrew - Package manager for BlackHole installation

## Data Storage

**Databases:**
- SQLite (embedded)
- Connection: Local file at app_data_dir/echo_note.db
- Client: SQLx with runtime-tokio and sqlite features
- Migrations: SQLx migrate! macro (src-tauri/migrations/)

**File Storage:**
- Local filesystem only
- Locations:
  - Models: ~/Library/Application Support/echo-note/models/
  - Recordings: ~/Library/Application Support/echo-note/recordings/
  - Database: ~/Library/Application Support/echo-note/echo_note.db

**Caching:**
- None (whisper models cached locally after download)

## Authentication & Identity

**Auth Provider:**
- None - Local desktop application
- No user accounts or authentication required

## Monitoring & Observability

**Error Tracking:**
- None - Logs to console/file only
- Uses `log` crate for structured logging

**Logs:**
- Rust `log` crate for backend logging
- Tauri event system for frontend-backend communication

## CI/CD & Deployment

**Hosting:**
- Local desktop application (not hosted)
- Distribution via Tauri bundler (macOS .app, Windows .exe, Linux AppImage)

**CI Pipeline:**
- None configured
- Local quality gates via Just commands:
  - `just check` - TypeScript + Rust type checking
  - `just lint` - Biome + Clippy
  - `just fmt` - Formatting
  - `just test-rs` - Rust unit tests

## Environment Configuration

**Required env vars:**
- None - pure desktop app with SQLite-based settings

**Settings stored in database:**
- audio_device - Selected microphone device ID
- whisper_model_size - Whisper model size (tiny/small/medium/large)
- llm_provider - "ollama" or "api"
- api_key - API key for cloud LLM providers
- api_endpoint - Custom API endpoint URL

**Secrets location:**
- SQLite database (app_data_dir/echo_note.db)
- Settings table with key-value storage
- No encryption at rest configured

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None - all integrations are pull-based

## Hardware Integration

**Audio Capture:**
- cpal library for microphone input
- BlackHole virtual driver for system audio (macOS only)
- Dual-channel mixing (mic + system audio when BlackHole available)

**Audio Processing:**
- hound for WAV file I/O
- Linear interpolation resampling (16kHz target)
- Mono channel mixing for multi-channel sources

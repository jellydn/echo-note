# External Integrations

## Databases
- SQLite via sqlx 0.8
- Local database file: `echo_note.db` in app data directory
- Tables: `meetings`, `transcripts`, `summaries`, `settings`
- Foreign key constraints enabled
- Connection pool: max 5 concurrent connections

## External APIs & Services
- OpenAI-compatible APIs (Chat Completions endpoint)
- API Key authentication via Bearer token
- Configurable endpoint URL (default: https://api.openai.com/v1)
- Model: gpt-4o-mini (configurable)
- Used for meeting summary generation as alternative to Ollama

## Local Services
- **Ollama** (localhost:11434) - LLM for meeting summaries
  - Default model: llama3.2
  - Endpoint: http://localhost:11434/api/generate
  - Fallback to Ollama if API provider not configured
  - Status check available via /api/tags endpoint

## System Integrations
- **Audio Input/Output**: CPAL (Cross-Platform Audio Library)
  - Microphone capture with configurable device selection
  - Multiple sample formats supported: F32, I16, U16
  - Default: 16kHz mono recording

- **Virtual Audio Driver**: BlackHole
  - System audio capture (optional, for recording system output)
  - Device detection via `system_profiler` command
  - Installation support via Homebrew or manual GitHub download
  - HAL driver detection at `/Library/Audio/Plug-Ins/HAL`

- **Speech Recognition**: Whisper (via whisper-rs 0.13)
  - Local on-device transcription
  - Multiple model sizes:
    - tiny (78MB), tiny-q5_1 (33MB)
    - base (149MB), base-q5_1 (60MB)
    - small (489MB), small-q5_1 (190MB)
    - medium (1.6GB), medium-q5_0 (539MB)
    - large-v3-turbo (1.6GB), large-v3-turbo-q5_0 (574MB)
  - Models downloaded from Hugging Face: https://huggingface.co/ggerganov/whisper.cpp
  - Default model size: small
  - Max audio chunk: 30 seconds (480k samples @ 16kHz)

- **macOS System Integration**:
  - App data directory managed by Tauri
  - System Profiler for audio device enumeration
  - Terminal integration for Homebrew package installation
  - Finder integration for opening model folders

- **File Format Support**:
  - WAV audio files (via hound crate)
  - 16-bit PCM format for recordings
  - Configurable sample rate and channel count

## Configuration Settings Storage
- Settings stored in SQLite `settings` table
- Configurable keys:
  - `audio_device` (default: "default")
  - `whisper_model_size` (default: "small")
  - `llm_provider` (default: "ollama", alternatives: "api")
  - `api_key` (OpenAI-compatible API key)
  - `api_endpoint` (default: https://api.openai.com/v1)

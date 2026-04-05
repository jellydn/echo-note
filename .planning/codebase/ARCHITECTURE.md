# Architecture

**Analysis Date:** 2026-04-06

## Pattern Overview

**Overall:** Tauri v2 Desktop App (Multi-Layer Architecture)

**Key Characteristics:**
- **Privacy-first**: All processing local (Whisper, Ollama), no cloud dependencies
- **Audio-centric**: Real-time audio capture with dual-source mixing (mic + system)
- **SQLite persistence**: Single-file database for meetings, transcripts, summaries
- **Async throughout**: Tokio runtime with blocking tasks for heavy ML work

## Layers

**Frontend (React):**
- Purpose: UI rendering and user interaction
- Location: `src/`
- Contains: React components, hooks, CSS
- Depends on: Tauri API (`@tauri-apps/api`)
- Used by: User directly

**Tauri Bridge:**
- Purpose: IPC between frontend and Rust backend
- Location: `src-tauri/src/lib.rs` (commands)
- Contains: Command handlers, request/response types
- Depends on: Business logic modules
- Used by: Frontend via `invoke()` calls

**Business Logic:**
- Purpose: Core application functionality
- Location: `src-tauri/src/*/mod.rs`
- Contains:
  - `audio/` - Recording and mixing (`AudioRecorder`)
  - `whisper/` - Transcription and model management
  - `system_audio/` - BlackHole integration
  - `db/` - Data access layer
- Depends on: External crates (cpal, whisper-rs, sqlx)
- Used by: Tauri commands

**Data Layer:**
- Purpose: Persistence and settings
- Location: `src-tauri/src/db/mod.rs`
- Contains: SQLx queries, migrations, CRUD operations
- Depends on: SQLite via SQLx
- Used by: Business logic, Tauri commands

## Data Flow

**Audio Recording Flow:**
1. User clicks "Record" → Frontend calls `start_recording_command`
2. Command reads audio device from settings
3. `AudioRecorder.start_recording()` spawns thread(s) with cpal
4. If BlackHole available, spawns second thread for system audio
5. User clicks "Stop" → `stop_recording_command` called
6. Recording threads signaled to stop via mpsc channel
7. Audio data collected, mixed (if dual source), saved as WAV
8. Meeting record created in database

**Transcription Flow:**
1. User selects meeting → Frontend calls `transcribe_audio_command`
2. Command loads Whisper model (blocking task on thread pool)
3. Audio file read, resampled to 16kHz if needed
4. Audio processed in 30-second chunks (memory management)
5. Progress events emitted to frontend via `app_handle.emit()`
6. Full text assembled and saved to `transcripts` table

**Settings Management Flow:**
1. Frontend requests setting via `get_setting_command`
2. Database queried, fallback to hardcoded default
3. Setting returned or default used
4. Updates via `set_setting_command` with UPSERT logic

**State Management:**
- **Frontend**: React `useState` for view navigation (`App.tsx`)
- **Backend**: Tauri `State<'_, AppStateExt>` with `Mutex<AudioRecorder>`
- **Database**: SQLite for persistent state (meetings, settings)

## Key Abstractions

**AudioRecorder:**
- Purpose: Manage audio recording thread(s)
- Location: `src-tauri/src/audio/mod.rs` (lines 63-277)
- Pattern: Thread-per-device with mpsc control channel
- Key methods: `start_recording()`, `stop_recording()`, `is_recording()`

**Database Models:**
- Purpose: Type-safe database records
- Location: `src-tauri/src/db/mod.rs`
- Types: `Meeting`, `Transcript`, `Summary`, `Setting`
- Pattern: SQLx `FromRow` derive for query mapping

**ApiResponse<T>:**
- Purpose: Consistent API envelope for all Tauri commands
- Location: `src-tauri/src/lib.rs` (lines 26-47)
- Pattern: Generic wrapper with `success`, `data`, `error` fields

**Whisper Model Management:**
- Purpose: Download and manage transcription models
- Location: `src-tauri/src/whisper/mod.rs`
- Pattern: Hugging Face download with progress events

## Entry Points

**Application Entry:**
- Location: `src-tauri/src/main.rs`
- Triggers: OS launches .app bundle
- Responsibilities: Delegates to `lib.rs` `run()` function

**Frontend Entry:**
- Location: `src/main.tsx`
- Triggers: Tauri webview loads index.html
- Responsibilities: ReactDOM root creation, `<App />` mount

**Main UI Entry:**
- Location: `src/App.tsx`
- Triggers: React root renders
- Responsibilities: View routing, navigation state

**Tauri Setup:**
- Location: `src-tauri/src/lib.rs` (lines 534-574, in `run()`)
- Triggers: App startup
- Responsibilities: DB initialization, default settings, state management

## Error Handling

**Strategy:** Result types throughout, user-friendly error messages

**Patterns:**
- Rust: `anyhow::Result` for internal errors, `Result<T, String>` for Tauri commands
- Error mapping: `.map_err(|e| format!("Context: {}", e))?`
- Frontend receives structured errors via `ApiResponse.error`
- Critical failures use `.expect()` (DB init, app setup)

## Cross-Cutting Concerns

**Logging:**
- Framework: `log` crate
- Pattern: `log::info!()` for progress, `log::warn!()` for non-fatal issues
- Events: Frontend receives progress via Tauri events (`transcription-progress`, `whisper-download-progress`)

**Validation:**
- Date parsing: RFC3339 format validation in `create_meeting_command`
- Model size: Validated against `WHISPER_MODELS` array
- Device matching: Partial string matching for BlackHole/microphone

**Authentication:**
- None - single-user desktop app
- API keys stored as plain strings in SQLite (security concern noted)

---

*Architecture analysis: 2026-04-06*

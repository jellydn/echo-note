# Architecture

**Analysis Date:** 2026-04-25

## Pattern Overview

**Overall:** Tauri v2 Desktop Application with Modular Backend

**Key Characteristics:**
- Hybrid architecture: React frontend + Rust backend
- Command-based RPC via Tauri invoke handler
- Thread-safe audio recording with dedicated audio threads
- Event-driven progress reporting for long-running operations
- Privacy-first: all processing local (Whisper, Ollama)
- SQLite persistence with SQLx compile-time checked queries

## Layers

**Frontend (Presentation Layer):**
- Purpose: React UI with view-based navigation
- Location: `src/`
- Contains: Components, hooks, styles
- Depends on: @tauri-apps/api for backend communication
- Used by: User interactions

**Tauri Bridge (IPC Layer):**
- Purpose: Command routing between frontend and backend
- Location: `src-tauri/src/lib.rs`
- Contains: `generate_handler!` macro, invoke handlers
- Depends on: All command modules
- Used by: Frontend via `invoke()` calls

**Commands (API Layer):**
- Purpose: Tauri command handlers exposing backend functionality
- Location: `src-tauri/src/commands/`
- Contains: 8 command modules (audio, llm, meetings, settings, summaries, transcription, transcripts)
- Depends on: Domain modules (db, audio, whisper, llm, system_audio)
- Used by: Tauri bridge

**Domain (Business Logic Layer):**
- Purpose: Core business logic and external integrations
- Location: `src-tauri/src/{db,audio,whisper,llm,system_audio}/`
- Contains: Database operations, audio recording, Whisper transcription, LLM summarization, system audio detection
- Depends on: External crates (cpal, whisper-rs, sqlx, reqwest)
- Used by: Commands layer

**Data (Persistence Layer):**
- Purpose: SQLite database with migrations
- Location: `src-tauri/migrations/`
- Contains: SQL schema definitions
- Depends on: SQLx runtime
- Used by: db module

## Data Flow

**Recording & Processing Flow:**
1. User clicks "Record" in frontend
2. Frontend calls `start_recording_command` via Tauri invoke
3. Command accesses `AppStateExt` for recorder and settings
4. Audio module spawns dedicated recording threads (mic + optional BlackHole system audio)
5. User stops recording → `stop_recording_command` called
6. Audio module mixes streams, saves WAV file
7. Meeting record created via `create_meeting_command`
8. Transcription triggered via `transcribe_audio_command`
9. Whisper module loads model, processes audio, emits progress events
10. Transcript saved to database
11. Optional: Summary generation via `generate_summary_command` → Ollama API
12. Results returned to frontend, navigation to meeting detail

**Settings Flow:**
1. Frontend reads/writes settings via `get_setting_command` / `set_setting_command`
2. Settings stored in SQLite with defaults initialized on app startup
3. Audio device preference affects recording source selection

**State Management:**
- **Frontend:** React useState for local UI state, useEffect for Tauri event listeners
- **Backend:** `AppStateExt` struct managed by Tauri (`manage()`) containing:
  - `db: sqlx::Pool<Sqlite>` - database connection pool
  - `audio_recorder: Mutex<AudioRecorder>` - thread-safe recorder access
- **Audio Recording:** Separate thread per audio source (mic + system) with mpsc channels for control

## Key Abstractions

**AppStateExt:**
- Purpose: Shared application state accessible to all Tauri commands
- Examples: `src-tauri/src/lib.rs:15-18`
- Pattern: Tauri State management with interior mutability (Mutex)

**ApiResponse<T>:**
- Purpose: Consistent response wrapper for all Tauri commands
- Examples: `src-tauri/src/lib.rs:21-44`
- Pattern: Generic Result type with success flag, optional data, and optional error

**AudioRecorder:**
- Purpose: Thread-safe audio recording with dual-source mixing capability
- Examples: `src-tauri/src/audio/mod.rs:51-65`
- Pattern: Struct with interior state, dedicated threads per audio source

**Command Modules:**
- Purpose: Organize Tauri commands by domain
- Examples: `src-tauri/src/commands/audio.rs`, `src-tauri/src/commands/meetings.rs`
- Pattern: One module per domain, `#[tauri::command]` annotated functions

**Domain Modules:**
- Purpose: Encapsulate external integrations and business logic
- Examples: `src-tauri/src/whisper/mod.rs`, `src-tauri/src/llm/mod.rs`
- Pattern: Pure Rust modules with error handling via `anyhow::Result`

## Entry Points

**Application Bootstrap:**
- Location: `src-tauri/src/main.rs`
- Triggers: Operating system launches the binary
- Responsibilities: Calls `echo_note_lib::run()` to start Tauri app

**Tauri Builder Setup:**
- Location: `src-tauri/src/lib.rs:52-75`
- Triggers: Application bootstrap
- Responsibilities: Initialize database, run migrations, set default settings, register state

**Command Registration:**
- Location: `src-tauri/src/lib.rs:76-120`
- Triggers: Tauri builder chain
- Responsibilities: Register all Tauri commands via `generate_handler!` macro

**Frontend Bootstrap:**
- Location: `src/main.tsx`
- Triggers: WebView loads index.html
- Responsibilities: Mount React app to DOM root

**Database Migrations:**
- Location: `src-tauri/migrations/0001_initial_schema.sql`
- Triggers: `sqlx::migrate!` in `db::init_db`
- Responsibilities: Create tables (meetings, transcripts, summaries, settings)

## Error Handling

**Strategy:** Layer-specific error handling with conversion to user-friendly strings at API boundary

**Patterns:**
- Backend uses `anyhow::Result` for ergonomic error propagation
- Commands convert errors to `String` via `map_err(|e| format!(...))`
- All commands return `Result<ApiResponse<T>, String>` for consistent frontend handling
- Frontend checks `response.success` flag before accessing `response.data`
- `ErrorBoundary` React component catches frontend render errors

## Cross-Cutting Concerns

**Logging:** Approach via `log` crate with structured logging throughout backend modules

**Validation:** Input validation at command boundaries, database constraints via SQLite

**Authentication:** None (privacy-first local app - no user accounts)

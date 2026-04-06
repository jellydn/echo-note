# Architecture

## Pattern
Layered architecture with event-driven IPC between frontend and backend:
- **Frontend Layer**: React 19 + TypeScript (Vite)
- **IPC Layer**: Tauri v2 commands (40+ async handlers) + Tauri event system
- **Backend Layer**: Rust (tokio async runtime) with modular subsystems
- **Data Layer**: SQLite via sqlx async driver

## System Layers

### Frontend (src/)
- React components using `@tauri-apps/api/core` invoke pattern
- State management via React hooks (`useState`, `useEffect`, `useCallback`)
- 4 main views: `RecordView`, `HistoryView`, `MeetingDetailView`, `SettingsView`
- CSS layout with sidebar navigation

### Backend (src-tauri/src/)
6 modular subsystems:
- `db/mod.rs` — SQLite ORM layer (meetings, transcripts, summaries, settings)
- `audio/mod.rs` — Dual-stream audio recording (mic + system audio via CPAL)
- `whisper/mod.rs` — Transcription (whisper-rs with quantized models)
- `llm/mod.rs` — Summary generation (Ollama or OpenAI-compatible APIs)
- `system_audio/mod.rs` — BlackHole virtual audio device detection
- `lib.rs` — 40+ Tauri command handlers coordinating all operations

### IPC Layer (Tauri)
- 40 async commands bridging frontend and backend
- Consistent `ApiResponse<T>` wrapper for all responses
- Command categories: Meetings, Transcripts, Summaries, Settings, Audio, Models, Transcription, LLM
- Tauri event system (`listen()`) for streaming progress updates (e.g., `"transcription-progress"`)

## Data Flow
```
Recording
  → Stop Recording
  → Create Meeting (SQLite)
  → Transcribe Audio (Whisper local)
  → Save Transcript (SQLite)
  → Generate Summary (Ollama / OpenAI API)
  → Save Summary (SQLite)
  → Display in History
```

## Entry Points
- **Frontend**: `src/main.tsx` → `src/App.tsx`
- **Backend**: `src-tauri/src/main.rs` → `src-tauri/src/lib.rs::run()`
- **Tauri config**: `src-tauri/tauri.conf.json`

## Key Abstractions
- `AppStateExt` — Manages DB pool + AudioRecorder (held in Tauri `State<>`)
- `ApiResponse<T>` — Consistent success/error response wrapper for all Tauri commands
- `AudioRecorder` — Manages mic + system audio threads with message passing
- `Meeting`, `Transcript`, `Summary`, `Setting` — Core domain models (serde Serialize/Deserialize)

## State Management
- **Frontend**: Local React hooks per component (no global state store)
- **Backend**: `tauri::State<AppStateExt>` with `Mutex`-protected `AudioRecorder`
- **Audio thread pattern**: `cpal::Stream` is not `Send`/`Sync` — dedicated audio thread with channel-based message passing

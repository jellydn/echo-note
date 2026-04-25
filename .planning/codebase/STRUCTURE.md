# Codebase Structure

**Analysis Date:** 2026-04-25

## Directory Layout

```
/Users/huynhdung/src/tries/2026-04-06-echo-note/
├── src/                    # React frontend (TypeScript)
│   ├── components/         # React view components
│   ├── assets/             # Static frontend assets
│   ├── App.tsx             # Main app shell with view routing
│   ├── App.css             # Global app styles
│   ├── main.tsx            # React entry point
│   └── vite-env.d.ts       # Vite type declarations
├── src-tauri/              # Rust backend (Tauri)
│   ├── src/                # Rust source code
│   │   ├── audio/          # Audio recording module
│   │   ├── commands/       # Tauri command handlers
│   │   ├── db/             # Database operations
│   │   ├── llm/            # LLM/Ollama integration
│   │   ├── system_audio/   # BlackHole system audio
│   │   ├── whisper/        # Whisper transcription
│   │   ├── lib.rs          # Library entry point
│   │   └── main.rs         # Binary entry point
│   ├── migrations/         # SQLx database migrations
│   ├── capabilities/       # Tauri capability definitions
│   ├── gen/                # Auto-generated (Tauri build)
│   ├── icons/              # App icons
│   ├── resources/          # Bundled resources
│   └── Cargo.toml          # Rust dependencies
├── scripts/                # Utility scripts
│   └── ralph/              # Autonomous agent scripts
├── tasks/                  # PRD and planning docs
├── public/                 # Static public assets
├── assets/                 # Additional assets
├── package.json            # Node.js dependencies
├── vite.config.ts          # Vite build configuration
├── tsconfig.json           # TypeScript configuration
├── biome.json              # Biome linting config
├── justfile                # Just task runner
└── index.html              # HTML entry point
```

## Directory Purposes

**src/ (Frontend):**
- Purpose: React TypeScript UI layer
- Contains: Components, styles, app shell
- Key files: `App.tsx`, `main.tsx`, `components/RecordView.tsx`

**src-tauri/src/commands/:**
- Purpose: Tauri command handlers (API layer)
- Contains: 8 command modules exposing backend functions
- Key files: `audio.rs`, `meetings.rs`, `transcription.rs`, `llm.rs`

**src-tauri/src/audio/:**
- Purpose: Audio capture and WAV file generation
- Contains: `AudioRecorder` struct, thread management, audio mixing
- Key files: `mod.rs` (592 lines)

**src-tauri/src/db/:**
- Purpose: SQLite database operations via SQLx
- Contains: CRUD functions for meetings, transcripts, summaries, settings
- Key files: `mod.rs` (479 lines)

**src-tauri/src/whisper/:**
- Purpose: Whisper.cpp integration for transcription
- Contains: Model management, download progress, transcription
- Key files: `mod.rs` (479 lines)

**src-tauri/src/llm/:**
- Purpose: Ollama and OpenAI-compatible API integration
- Contains: Summary generation, response parsing
- Key files: `mod.rs` (419 lines)

**src-tauri/src/system_audio/:**
- Purpose: BlackHole virtual audio driver detection
- Contains: Device detection, installation helpers
- Key files: `mod.rs` (234 lines)

**src-tauri/migrations/:**
- Purpose: Database schema versioning
- Contains: SQL migration files
- Key files: `0001_initial_schema.sql`

## Key File Locations

**Entry Points:**
- `src/main.tsx`: React DOM mounting
- `src-tauri/src/main.rs`: Rust binary entry
- `src-tauri/src/lib.rs`: Tauri app builder and setup
- `index.html`: WebView HTML shell

**Configuration:**
- `package.json`: Frontend dependencies (React, Tauri API)
- `src-tauri/Cargo.toml`: Rust dependencies (Tauri, SQLx, Whisper)
- `vite.config.ts`: Vite dev server (port 1420)
- `biome.json`: Code quality configuration
- `justfile`: Development tasks

**Core Logic:**
- `src/App.tsx`: View routing (record/history/settings/meeting-detail)
- `src/components/RecordView.tsx`: Recording workflow orchestration
- `src-tauri/src/audio/mod.rs`: Audio thread management
- `src-tauri/src/db/mod.rs`: All database operations

**Testing:**
- Inline tests in Rust modules (e.g., `whisper/mod.rs` tests section)
- No dedicated test directories found

## Naming Conventions

**Files:**
- React components: PascalCase (`RecordView.tsx`, `ErrorBoundary.tsx`)
- React styles: `ComponentName.css` pattern
- Rust modules: snake_case (`mod.rs` in named directories)
- Rust commands: `*_command` suffix (`start_recording_command`)

**Directories:**
- Frontend: lowercase (`components/`, `assets/`)
- Rust modules: snake_case (`system_audio/`, `commands/`)
- Tauri standard: `src-tauri/`, `migrations/`, `capabilities/`

**Rust Conventions:**
- Structs: PascalCase (`AudioRecorder`, `AppStateExt`)
- Functions: snake_case (`start_recording`, `get_models_dir`)
- Constants: SCREAMING_SNAKE_CASE (`DEFAULT_OLLAMA_URL`)
- Modules declared in `lib.rs` with `mod module_name;`

## Where to Add New Code

**New Feature:**
- Primary code: Add command in `src-tauri/src/commands/new_feature.rs`
- Register command: Add to `generate_handler!` in `src-tauri/src/lib.rs`
- Frontend UI: Add to appropriate component in `src/components/`
- Tests: Inline in module or new `#[cfg(test)]` module

**New Component/Module:**
- Implementation: `src-tauri/src/new_module/mod.rs`
- Declaration: Add `mod new_module;` to `src-tauri/src/lib.rs`
- Commands: Create `src-tauri/src/commands/new_module.rs`

**Utilities:**
- Shared helpers: Re-export from `src-tauri/src/lib.rs` or create `utils/` module
- Frontend utilities: Add to `src/utils/` (directory doesn't exist, create if needed)

## Special Directories

**src-tauri/gen/:**
- Purpose: Auto-generated code from Tauri build process
- Generated: Yes (by `cargo tauri dev` or `build`)
- Committed: No (typically in .gitignore, but currently tracked)

**src-tauri/capabilities/:**
- Purpose: Tauri v2 capability definitions for permissions
- Generated: Partially
- Committed: Yes

**src-tauri/migrations/:**
- Purpose: SQLx database migrations
- Generated: No (hand-written)
- Committed: Yes
- Important: Embedded via `sqlx::migrate!` macro

**scripts/ralph/:**
- Purpose: Autonomous agent scripts for development workflow
- Generated: No
- Committed: Yes

**node_modules/:**
- Purpose: Node.js dependencies
- Generated: Yes (by `bun install`)
- Committed: No (in .gitignore)

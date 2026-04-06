# Directory Structure

## Top-Level Layout
```
2026-04-06-echo-note/
├── src/                    # React frontend (TypeScript)
├── src-tauri/              # Rust backend (Tauri)
│   ├── src/                # Rust source files
│   ├── migrations/         # SQLx database migrations
│   ├── gen/                # Auto-generated Tauri schemas (don't edit)
│   ├── Cargo.toml          # Rust dependencies
│   ├── tauri.conf.json     # Tauri app configuration
│   └── build.rs            # Build script
├── tasks/                  # PRD and planning docs
├── scripts/ralph/          # Autonomous agent scripts
├── .planning/codebase/     # Codebase documentation (this directory)
├── package.json            # Node.js dependencies & scripts
├── vite.config.ts          # Frontend build configuration
├── tsconfig.json           # TypeScript configuration
├── biome.json              # Linting and formatting rules
└── justfile                # Convenience commands (just check, just lint, etc.)
```

## Frontend (src/)
```
src/
├── main.tsx                # React app entry point (10 lines)
├── App.tsx                 # View router + sidebar navigation (104 lines)
├── App.css                 # Global styles
└── components/
    ├── RecordView.tsx       # Recording UI (566 lines)
    ├── HistoryView.tsx      # Meetings list (80+ lines)
    ├── MeetingDetailView.tsx # Meeting details view
    └── SettingsView.tsx     # Settings panel (680 lines)
```

## Backend (src-tauri/)
```
src-tauri/
├── src/
│   ├── main.rs             # Entry point — delegates to lib.rs (7 lines)
│   ├── lib.rs              # All 40 Tauri commands + app setup (1087 lines)
│   ├── db/
│   │   └── mod.rs          # Database schema, queries, CRUD (540 lines)
│   ├── audio/
│   │   └── mod.rs          # Dual-stream audio recording via CPAL (568 lines)
│   ├── whisper/
│   │   └── mod.rs          # Local transcription via whisper-rs
│   ├── llm/
│   │   └── mod.rs          # Summary generation (Ollama + OpenAI)
│   └── system_audio/
│       └── mod.rs          # BlackHole driver detection
├── migrations/             # SQLx migration files (.sql)
├── gen/                    # Auto-generated (do not edit)
└── tauri.conf.json         # Window, bundle, security config
```

## Configuration & Scripts
- `justfile` — Convenience aliases: `just check`, `just lint`, `just fmt`, `just pre-commit`
- `scripts/ralph/` — Ralph autonomous agent scripts + `prd.json` + `progress.txt`
- `tasks/` — PRD documents and planning artifacts
- `.planning/codebase/` — This codebase map

## Naming Conventions

### Files
- React components: `PascalCase.tsx` (e.g., `RecordView.tsx`, `SettingsView.tsx`)
- Rust modules: `snake_case/mod.rs` (e.g., `audio/mod.rs`, `system_audio/mod.rs`)
- Migrations: numbered SQL files in `migrations/`

### Code
- React components: `PascalCase` named exports
- Hooks: `camelCase` starting with `use`
- Rust functions: `snake_case`
- Rust types/structs/enums: `PascalCase`
- Rust constants: `SCREAMING_SNAKE_CASE`
- Tauri commands: `snake_case` (e.g., `start_recording`, `get_meeting`)
- Database tables: `snake_case`, plural (e.g., `meetings`, `transcripts`)

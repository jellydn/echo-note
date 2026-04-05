# Technology Stack

**Analysis Date:** 2026-04-06

## Languages

**Primary:**
- TypeScript 5.8.3 - React frontend (`src/`)
- Rust Edition 2021 - Tauri backend (`src-tauri/src/`)
- SQL - Database queries (embedded in Rust via SQLx)

**Secondary:**
- CSS - Component styling
- JSON - Configuration files

## Runtime

**Environment:**
- Tauri v2 - Desktop app framework
- Vite 7.0.4 - Frontend dev server and build tool
- Node.js (via bun) - Frontend runtime

**Package Manager:**
- Bun (frontend) - `bun install`
- Cargo (Rust) - `cargo build`
- Lockfile: `Cargo.lock` present, `bun.lockb` present

## Frameworks

**Core:**
- React 19.1.0 - UI framework (`src/App.tsx`)
- Tauri v2 - Desktop app shell and system integration
- SQLx 0.8 - Async SQLite ORM (`src-tauri/src/db/mod.rs`)

**Audio/ML:**
- cpal 0.15 - Cross-platform audio capture (`src-tauri/src/audio/mod.rs`)
- whisper-rs 0.13 - Whisper.cpp bindings for transcription (`src-tauri/src/whisper/mod.rs`)

**Build/Dev:**
- Vite 7.0.4 - Frontend bundling and dev server
- @vitejs/plugin-react 4.6.0 - React HMR support
- tauri-build v2 - Rust build integration

## Key Dependencies

**Critical:**
- `whisper-rs` 0.13 - Local Whisper transcription (privacy-first, no cloud)
- `cpal` 0.15 - Audio capture from microphone and system
- `sqlx` 0.8 - Compile-time checked SQL with SQLite
- `tokio` 1.x - Async runtime for Rust
- `@tauri-apps/api` ^2 - Frontend-to-backend bridge

**Infrastructure:**
- `serde` 1.x - Serialization for Tauri commands
- `chrono` 0.4 - Date/time handling
- `anyhow` 1.x - Error handling
- `hound` 3.5 - WAV file writing
- `reqwest` 0.12 - HTTP client for model downloads
- `futures-util` 0.3 - Streaming downloads

**Development:**
- `@biomejs/biome` ^2.4.10 - Linting and formatting
- `typescript` ~5.8.3 - Type checking

## Configuration

**Environment:**
- No `.env` files currently
- Settings stored in SQLite database (`settings` table)
- Default settings defined in `src-tauri/src/db/mod.rs`

**Build:**
- `package.json` - Frontend dependencies and scripts
- `src-tauri/Cargo.toml` - Rust dependencies
- `src-tauri/tauri.conf.json` - Tauri app configuration
- `tsconfig.json` - TypeScript configuration
- `vite.config.ts` (implied) - Vite configuration

**Quality Tools:**
- Biome configured in `package.json` scripts (no separate config file)
- rustfmt defaults (no custom configuration)
- Clippy for Rust linting

## Platform Requirements

**Development:**
- macOS (primary target platform)
- Bun package manager
- Rust toolchain
- Tauri CLI (`cargo tauri`)

**Production:**
- macOS desktop app (`.app` bundle)
- BlackHole virtual audio driver for system audio capture
- Local Ollama instance (optional, port 11434)
- ~250MB-1GB disk space for Whisper models

---

*Stack analysis: 2026-04-06*

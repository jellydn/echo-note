# Technology Stack

**Analysis Date:** 2026-04-25

## Languages

**Primary:**
- TypeScript ~5.8.3 - React frontend (src/)
- Rust Edition 2021 - Tauri backend (src-tauri/src/)

**Secondary:**
- SQL - Database migrations (src-tauri/migrations/)

## Runtime

**Environment:**
- Tauri v2 - Desktop application framework
- Node.js/Bun - Frontend tooling and package management

**Package Manager:**
- Bun (JavaScript/TypeScript dependencies)
- Cargo (Rust dependencies)
- Lockfile: bun.lock (present), Cargo.lock (present)

## Frameworks

**Core:**
- React ^19.1.0 - UI component library
- Tauri v2 - Cross-platform desktop app runtime (Rust backend + Web frontend)
- SQLx 0.8 - Async SQL toolkit with compile-time checked queries
- Tokio 1.x - Async runtime for Rust

**Testing:**
- Built-in Rust test framework (`cargo test`)
- No dedicated JS test framework configured

**Build/Dev:**
- Vite ^7.0.4 - Frontend build tool and dev server
- Tauri CLI ^2 - App building and packaging
- Biome ^2.4.10 - Linting and formatting (TypeScript/JSON)

## Key Dependencies

**Critical:**
- whisper-rs 0.13 - OpenAI Whisper transcription (Rust bindings)
- cpal 0.15 - Cross-platform audio capture library
- sqlx 0.8 - SQLite database with compile-time query validation
- reqwest 0.12 - HTTP client for API calls (Ollama, HuggingFace)
- hound 3.5 - WAV file reading/writing

**Infrastructure:**
- serde + serde_json - Serialization
- chrono 0.4 - Date/time handling
- anyhow 1.x - Error handling
- futures-util 0.3 - Async streaming (model downloads)
- tauri-plugin-opener 2 - File/URL opening

## Configuration

**Environment:**
- No .env files (desktop app, settings stored in SQLite)
- Settings table in database for user configuration
- Default settings: audio_device, whisper_model_size, llm_provider, api_key, api_endpoint

**Build:**
- vite.config.ts - Vite configuration (port 1420, Tauri-specific settings)
- tsconfig.json - TypeScript strict mode, ES2020 target
- tsconfig.node.json - Node-specific TS config (referenced)
- biome.json - Linting/formatting rules (tabs, double quotes, trailing commas)
- tauri.conf.json - App metadata, window config, security CSP, bundle settings

## Platform Requirements

**Development:**
- macOS (system_audio module uses macOS-specific APIs)
- Bun package manager
- Rust toolchain (cargo, clippy, fmt)
- Xcode Command Line Tools (for BlackHole detection)
- Just (optional) - Task runner for quality gates

**Production:**
- macOS app bundle (.app)
- Targets: all platforms (configured in tauri.conf.json)
- Bundle includes: icons, native binaries, embedded frontend

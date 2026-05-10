# Technology Stack

**Analysis Date:** 2026-05-06

EchoNote is a privacy-first native desktop app built on **Tauri v2** (Rust backend) with a **React 19 + TypeScript** frontend. It records meetings, transcribes audio locally via Whisper (`whisper-rs`), and summarizes via Ollama or an OpenAI-compatible cloud API.

## Languages

**Primary:**
- **Rust** edition `2021` — Tauri backend, audio capture, transcription, LLM client, SQLite DAL (`src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`)
- **TypeScript** `~5.8.3` (strict mode, `target: ES2020`, `jsx: react-jsx`) — React frontend in `src/` (`tsconfig.json`, `package.json`)

**Secondary:**
- **SQL** (SQLite dialect) — schema in `src-tauri/migrations/0001_initial_schema.sql`
- **JSON** for Tauri configuration and capabilities (`src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`)
- **Shell / AppleScript** invoked from Rust to drive Homebrew + Terminal flows for BlackHole install (`src-tauri/src/system_audio/mod.rs:200`)
- **Justfile** task runner DSL (`justfile`)

## Runtime

**Environment:**
- **Tauri v2** desktop runtime (macOS-first; AppleScript / `osascript` paths in `src-tauri/src/system_audio/mod.rs` indicate macOS as primary target)
- **Tokio** async runtime with `features = ["full"]` for the Rust backend (`src-tauri/Cargo.toml`)
- Browser runtime: WebView2/WKWebView served by Tauri; dev server on `http://localhost:1420` (`src-tauri/tauri.conf.json`, `vite.config.ts`)

**Package Manager:**
- **Bun** for JS deps (`bun.lock` present, `tauri.conf.json` uses `bun run dev` / `bun run build` as Tauri before-hooks; `justfile` uses `bun install`)
- **Cargo** for Rust deps (`src-tauri/Cargo.toml`); lockfile present at `src-tauri/Cargo.lock` (assumed standard)
- Lockfiles: `bun.lock` ✅ present; `package.json` lists `npm`-style scripts but Bun is the canonical installer

## Frameworks

**Core:**
- **Tauri** `^2` (`tauri`, `tauri-build`, `@tauri-apps/api`, `@tauri-apps/cli`) — desktop shell, IPC via `#[tauri::command]`, event emission via `app_handle.emit(...)` (`src-tauri/src/lib.rs`, `src-tauri/src/whisper/mod.rs:170`)
- **React** `^19.1.0` + **react-dom** `^19.1.0` — UI layer (`src/main.tsx`, `package.json`)
- **SQLx** `0.8` with `runtime-tokio`, `sqlite`, `chrono`, `migrate` features — async DB access and embedded migrations (`src-tauri/Cargo.toml`)

**Testing:**
- **Vitest** `^3.2.3` with `@vitest/coverage-v8` — unit tests in `src/**/*.{test,spec}.{ts,tsx}` using `jsdom` environment (`vitest.config.ts`, `src/test/setup.ts`)
- **@testing-library/react** `^16.3.0`, **@testing-library/jest-dom** `^6.6.3`, **@testing-library/user-event** `^14.6.1` — React component testing (`package.json`)
- **Cargo test** — Rust unit tests, e.g. `assert_eq!(DEFAULT_OLLAMA_URL, "http://localhost:11434")` in `src-tauri/src/llm/mod.rs:416`; runner alias `just test-rs` (`justfile`)

**Build / Dev:**
- **Vite** `^7.0.4` with `@vitejs/plugin-react` `^4.6.0` — fixed dev port `1420`, strict port enforcement, ignores `src-tauri/**` (`vite.config.ts`)
- **TypeScript compiler** `tsc --noEmit` for typecheck; project references `tsconfig.node.json` (`tsconfig.json`)
- **Biome** `^2.4.10` — formatter + linter, tab indent, line width `100`, double quotes, trailing commas all (`biome.json`)
- **Clippy** with `-D warnings` and **rustfmt** (`justfile`)
- **prek** pre-commit hook runner (`prek.toml`) — runs trailing-whitespace, EOF, large-file check, biome-check, tsc, cargo fmt, cargo clippy
- **just** task runner (`justfile`) — `dev`, `build`, `check`, `lint`, `fmt`, `pre-commit`, `test-rs`, `setup`

## Key Dependencies

**Critical (Rust — `src-tauri/Cargo.toml`):**
- `tauri = "2"` — desktop app shell + IPC
- `tauri-plugin-opener = "2"` — opens external URLs (used to launch BlackHole release page in `src-tauri/src/system_audio/mod.rs:173` and capability `opener:default` in `src-tauri/capabilities/default.json`)
- `sqlx = "0.8"` (sqlite + tokio + chrono + migrate) — DB pool, query, migrations
- `tokio = "1"` — async runtime (`features = ["full"]`)
- `cpal = "0.15"` — cross-platform audio I/O for microphone + BlackHole capture (`src-tauri/src/audio/mod.rs`, `src-tauri/src/system_audio/mod.rs`)
- `hound = "3.5"` — WAV file encoding for recorded audio
- `whisper-rs = "0.13"` — local Whisper inference via whisper.cpp bindings (`src-tauri/src/whisper/mod.rs`)
- `reqwest = "0.12"` (`stream`, `json`) — HTTP client for Ollama, OpenAI-compatible API, and Hugging Face model downloads (`src-tauri/src/llm/mod.rs:107`, `src-tauri/src/whisper/mod.rs:127`)
- `futures-util = "0.3"` — streaming download chunks (`src-tauri/src/whisper/mod.rs`)
- `serde = "1"` (derive) + `serde_json = "1"` — IPC payload (de)serialization
- `chrono = "0.4"` (serde) — timestamps on DB rows
- `anyhow = "1"` — error context
- `log = "0.4"` — backend logging (forwarded to Tauri default logger)
- `url = "2"` — URL parsing
- `uuid = "1"` (`v4`) — identifiers

**Critical (Frontend — `package.json`):**
- `@tauri-apps/api ^2` — `invoke()` IPC + event listeners
- `@tauri-apps/plugin-opener ^2` — opener plugin client
- `react ^19.1.0`, `react-dom ^19.1.0`

**Infrastructure:**
- `@biomejs/biome ^2.4.10` — lint/format
- `@vitejs/plugin-react ^4.6.0`, `vite ^7.0.4`
- `jsdom ^26.1.0` — DOM emulation for Vitest
- `@types/react ^19.1.8`, `@types/react-dom ^19.1.6`
- `tauri-build = "2"` (build dep) — generates Tauri context

## Configuration

**Environment:**
- No `.env` consumed by frontend code; runtime knobs are stored in the SQLite `settings` table (`src-tauri/migrations/0001_initial_schema.sql`, defaults in `src-tauri/src/db/mod.rs:78-83`):
  - `audio_device` = `default`
  - `whisper_model_size` = `small`
  - `llm_provider` = `ollama` (alt: `api`)
  - `api_key` = `""`
  - `api_endpoint` = `https://api.openai.com/v1`
  - `api_model` = `gpt-4o-mini`
- `TAURI_DEV_HOST` env var optionally consumed by Vite for LAN HMR (`vite.config.ts`)
- Whisper models stored under app data dir `models/` (per `AGENTS.md`: `~/Library/Application Support/echo-note/models/`)
- `dotenv-load` enabled in `justfile` (loads any local `.env` for recipes)

**Build:**
- `src-tauri/tauri.conf.json` — productName, identifier `com.huynhdung.echo-note`, window 800×600, bundle targets `all`, icon set, CSP whitelisting `'self' http://localhost:* https://api.openai.com https://*.openai.com` for `connect-src`
- `src-tauri/capabilities/default.json` — permissions `core:default`, `opener:default` for window `main`
- `vite.config.ts` — fixed port 1420, watch ignores `src-tauri/**`
- `vitest.config.ts` — jsdom env, coverage on `src/components/**/*.tsx`
- `tsconfig.json` / `tsconfig.node.json` — strict, bundler resolution, project references
- `biome.json` — excludes `node_modules`, `dist`, `target`, `.git`, `.vscode`, `.claude`, `src-tauri/gen`, lockfiles
- `renovate.json` — extends `config:recommended` for automated dep updates

## Platform Requirements

**Development:**
- macOS (BlackHole detection paths under `/Library/Audio/Plug-Ins/HAL` and `osascript` Terminal automation in `src-tauri/src/system_audio/mod.rs` are macOS-specific)
- Bun (or npm-compatible) for JS deps
- Rust toolchain with `cargo`, `cargo fmt`, `clippy`
- `just` task runner; `prek` for pre-commit hooks
- Tauri v2 prerequisites (Xcode CLI tools on macOS)
- Optional: **Homebrew** to install `blackhole-2ch`; **Ollama** running on `localhost:11434` for local LLM summaries

**Production:**
- Bundle target: `"all"` in `src-tauri/tauri.conf.json` (DMG/app bundle on macOS; Tauri's standard bundler picks platform-appropriate artifacts)
- Bundle ID: `com.huynhdung.echo-note`
- Single-window desktop app; no server-side hosting required
- Required runtime services: optional **Ollama** (local) and/or **OpenAI-compatible** cloud API; **BlackHole 2ch** virtual audio driver for system-audio capture

---

*Stack analysis: 2026-05-06*

# 🤖 Agent Guidelines for EchoNote

EchoNote is a privacy-first native desktop app (macOS) built with **Tauri v2 + React (TypeScript) + Rust**. It records meeting audio, transcribes locally via Whisper, and generates summaries via Ollama.

## Build & Development Commands

```bash
# Install frontend dependencies
bun install

# Development - launches Tauri app with hot reload
cargo tauri dev

# Typecheck (Required before commits)
cargo check                          # Rust typecheck
bun run typecheck                    # TypeScript typecheck

# Lint
bun run lint                         # Biome
cargo clippy                         # Rust linting
cargo fmt --check                    # Rust format check

# Build release
cargo tauri build

# Run Rust tests
cargo test <test_name>             # Run specific test

# Alternative: use just for convenience commands
just check                         # Run all checks
just lint                          # Run all lints
just fmt                           # Format everything

# No frontend tests exist yet
```

## Code Style Guidelines

### TypeScript / React

- **Use TypeScript strictly** - all props, state, and functions typed
- **Prefer explicit types** over `any` - use `unknown` when type is truly unknown
- **Component naming**: PascalCase for components (e.g., `RecordView.tsx`)
- **Hook naming**: camelCase starting with `use` (e.g., `useRecording`)
- **File naming**: Match the main export (component = PascalCase, utilities = camelCase)
- **Imports order**: React → third-party → internal (absolute) → relative (siblings)
- **Use named exports** for components and utilities

### Rust

- **Follow rustfmt defaults** - no custom configuration
- **Function naming**: `snake_case` for functions and variables
- **Type naming**: `PascalCase` for structs, enums, traits
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Error handling**: Use `anyhow` for errors, propagate with `?` operator
- **Prefer `Result` over panics** - use `expect()` only for truly unrecoverable cases

### Error Handling

- **Frontend**: Use try/catch with typed error messages, show user-friendly toasts
- **Backend (Rust)**: Return `Result<T, String>` from Tauri commands, never panic
- **Always log errors** with context before returning to frontend

### Naming Conventions

- **Tauri commands**: `snake_case` (e.g., `start_recording`, `get_meeting`)
- **Database tables**: `snake_case`, plural (e.g., `meetings`, `transcripts`)
- **API responses**: camelCase for JSON keys
- **File paths**: Use `PathBuf` in Rust, handle cross-platform separators

### Database (SQLite via SQLx)

- Use migrations in `src-tauri/migrations/`
- Always use `IF NOT EXISTS` for table creation
- Use `sqlx::query!` macro for compile-time SQL checking
- Keep queries in dedicated module (`src-tauri/src/db/`)

### State Management

- **React state**: Use hooks for local state, React Context for global state
- **Tauri state**: Use `tauri::State` for shared resources (DB pool, audio recorder)
- Avoid prop drilling - use context when props go >3 levels deep

### Testing

- **Rust**: Write unit tests in `#[cfg(test)]` modules
- **React**: Use React Testing Library, test user interactions not implementation
- **Integration**: Test Tauri commands through the frontend E2E flow

## Tauri Command Patterns

```rust
// Good: Returns Result with user-friendly error
#[tauri::command]
async fn start_recording(state: State<'_, AppState>) -> Result<String, String> {
    state.recorder.start()
        .map_err(|e| format!("Failed to start recording: {}", e))?;
    Ok("Recording started".to_string())
}
```

## Project Structure

```
src/              # React frontend (TypeScript)
src-tauri/        # Rust backend
  src/
    db/           # Database queries and migrations
    gen/          # Auto-generated Tauri schemas (don't edit)
  migrations/     # SQLx migrations
tasks/            # PRD and planning docs
scripts/ralph/    # Autonomous agent scripts
```

## Quality Gates (Required for Commits)

1. **Typecheck passes**: `cargo check && bun run typecheck`
2. **Lint clean**: `cargo clippy` (no warnings) and `bun run lint`
3. **Format check**: `cargo fmt --check` passes
4. **Single story focus**: Each commit addresses ONE user story from PRD
5. **Pre-commit hooks**: Run `just pre-commit` to verify locally (uses prek)

## Commit Message Format

```
feat: [US-XXX] - [Story Title]
```

Example: `feat: US-001 - Initialize Tauri + React project scaffold`

## Useful Context

- **Audio capture**: Uses `cpal` crate, saves WAV to app data directory
- **Audio thread pattern**: cpal `Stream` is not `Send`/`Sync` - use dedicated audio thread with message passing instead of storing in Tauri state
- **Transcription**: Uses `whisper-rs` (whisper.cpp bindings), models stored in `~/Library/Application Support/echo-note/models/`
- **LLM**: Ollama runs locally on port 11434, falls back to API if configured
- **Database**: SQLite at app data directory, use SQLx for compile-time checked queries
- **System audio**: Bundles BlackHole driver, mixes mic + system audio

## Ralph Agent Workflow

This project uses the Ralph autonomous agent. See `scripts/ralph/`:

- Read `prd.json` for current user stories
- Read `progress.txt` for learned patterns
- Update `progress.txt` with learnings after each story
- Check `passes: false` stories, work on highest priority
- Signal completion with `<promise>COMPLETE</promise>` when all stories pass

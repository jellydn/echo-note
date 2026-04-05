# Coding Conventions

**Analysis Date:** 2026-04-06

## Naming Patterns

**Files:**
- React components: PascalCase matching export (e.g., `App.tsx` exports `App`)
- Rust modules: `snake_case` directory with `mod.rs` (e.g., `audio/mod.rs`)
- Type definitions: Co-located or `types.ts` (not currently used)

**Functions:**
- Rust: `snake_case` (e.g., `start_recording`, `get_meeting_command`)
- TypeScript: `camelCase` (e.g., `setCurrentView`, `renderView`)
- React components: PascalCase (e.g., `App`, `RecordView`)

**Variables:**
- Rust: `snake_case` (e.g., `audio_recorder`, `db_pool`)
- TypeScript: `camelCase` (e.g., `currentView`, `device_infos`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_WHISPER_MODEL_SIZE`)

**Types:**
- Rust: PascalCase (e.g., `AudioRecorder`, `MeetingResponse`)
- TypeScript: PascalCase with explicit types (e.g., `View = "record" | "history"`)

## Code Style

**Formatting:**
- TypeScript: Biome (configured in `package.json` scripts, no separate config)
- Rust: rustfmt with defaults (no custom `.rustfmt.toml`)
- Indent: 4 spaces (observed in `App.tsx` tabs)

**Linting:**
- TypeScript: Biome `bun run lint`
- Rust: Clippy `cargo clippy` (no warnings allowed per AGENTS.md)
- Format check: `cargo fmt --check` and Biome check

**Quality Gates (per AGENTS.md):**
```bash
cargo check && bun run typecheck  # Type checking
bun run lint                      # Biome
cargo clippy                      # Rust lint
cargo fmt --check                 # Rust format
```

## Import Organization

**TypeScript (observed in `App.tsx`):**
```typescript
import { useState } from "react";     // React first
import "./App.css";                    // Relative styles
```

**Expected pattern (per AGENTS.md):**
1. React
2. Third-party
3. Internal (absolute)
4. Relative (siblings)

**Rust (observed in `lib.rs`):**
- Standard library: `use std::...`
- External crates: `use serde::...`, `use tauri::...`
- Internal modules: `use crate::audio::...`
- Grouped logically with blank lines between groups

## Error Handling

**Patterns:**
- Rust: `anyhow::Result` internally, `Result<T, String>` for Tauri commands
- Mapping: `.map_err(|e| format!("Failed to X: {}", e))?`
- TypeScript: Try/catch with typed error messages (to be implemented)

**Example from `lib.rs`:**
```rust
#[tauri::command]
async fn start_recording_command(...) -> Result<ApiResponse<bool>, String> {
    let device_id = get_setting(&state.db, "audio_device", DEFAULT_AUDIO_DEVICE)
        .await
        .map_err(|e| format!("Failed to get audio device setting: {}", e))?;
    // ...
}
```

## Logging

**Framework:** `log` crate (Rust)

**Patterns:**
- `log::info!()` for progress (e.g., "Download progress: {}%")
- `log::warn!()` for non-fatal issues (e.g., "Failed to emit download progress")
- `eprintln!()` for stream errors in audio thread

**Frontend:**
- Console logging not yet implemented
- Tauri events used for progress (`transcription-progress`)

## Comments

**When to Comment:**
- Module-level docs for public APIs
- Complex audio mixing/resampling logic
- Tauri command documentation

**Example from `audio/mod.rs`:**
```rust
/// Mix two audio streams together
/// If sample rates differ, the second stream is resampled to match the first
fn mix_audio_streams(...) -> Vec<f32> { ... }
```

## Function Design

**Size:**
- Tauri commands: ~10-30 lines (good)
- `transcribe_audio()`: ~150 lines (long, could refactor)
- `run_single_recording_thread()`: ~80 lines (acceptable for complexity)

**Parameters:**
- Prefer structs for multiple params (e.g., `CreateMeetingRequest`)
- State passed as `State<'_, AppStateExt>`

**Return Values:**
- Always use `ApiResponse<T>` wrapper for consistency
- Database IDs returned as `i64`

## Module Design

**Exports:**
- Rust: Explicit re-exports in `lib.rs` with `use crate::...`
- TypeScript: Default exports for components (`export default App`)

**Barrel Files:**
- Not used currently
- Each module accessed directly via `mod.rs`

## TypeScript-Specific

**Strictness:**
- All props, state, functions typed (per AGENTS.md)
- Prefer explicit types over `any`
- Use `unknown` when type is truly unknown

**React Patterns:**
- Functional components with hooks
- `useState` for local state
- Type unions for view state: `type View = "record" | "history" | "settings"`

---

*Convention analysis: 2026-04-06*

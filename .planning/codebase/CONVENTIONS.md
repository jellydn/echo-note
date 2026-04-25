# Coding Conventions

**Analysis Date:** 2026-04-25

## Naming Patterns

**Files:**
- React components: PascalCase (e.g., `RecordView.tsx`, `ErrorBoundary.tsx`)
- React components have default export at end: `export default ComponentName;`
- Rust modules: snake_case (e.g., `audio/mod.rs`, `commands/meetings.rs`)
- Rust command files named after domain (e.g., `meetings.rs`, `transcription.rs`)

**Functions:**
- TypeScript: camelCase for regular functions, PascalCase for React components
- Rust: snake_case for functions (e.g., `create_meeting_command`, `get_models_dir`)
- Tauri commands use `_command` suffix (e.g., `start_recording_command`)

**Variables:**
- TypeScript: camelCase (e.g., `audioDevice`, `isLoading`)
- Rust: snake_case (e.g., `audio_data`, `sample_rate`)
- Constants: UPPER_SNAKE_CASE (e.g., `DEFAULT_OLLAMA_URL`, `SETTING_AUDIO_DEVICE`)
- React refs use `Ref` suffix (e.g., `unlistenRef`, `recordingIntervalRef`)

**Types:**
- TypeScript interfaces: PascalCase with descriptive names (e.g., `ApiResponse<T>`, `MeetingResponse`)
- TypeScript types for props: `{ComponentName}Props` (e.g., `RecordViewProps`)
- Type aliases for state unions: PascalCase (e.g., `RecordingState`, `ProcessingStage`)
- Rust structs: PascalCase (e.g., `AppStateExt`, `RecordingResult`)
- Rust traits: PascalCase with descriptive names

## Code Style

**Formatting:**
- Tool: Biome (v2.4.10)
- Indent: Tab (not spaces)
- Line width: 100 characters
- Quotes: Double quotes for JavaScript/TypeScript
- Semicolons: Always required
- Trailing commas: All (e.g., `[1, 2, 3,]`)

**Linting:**
- TypeScript: Biome with recommended rules
- Key rule: `noUnusedVariables` set to "error"
- TypeScript strict mode enabled (`strict: true`)
- `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch` enabled
- Rust: Clippy with `-D warnings` (deny all warnings)
- Rust formatting: `cargo fmt` with standard settings

## Import Organization

**Order:**
1. Third-party libraries (e.g., `react`, `@tauri-apps/api/core`)
2. Tauri plugins and API
3. Relative imports (e.g., `./components/ErrorBoundary`)
4. CSS imports (always last)

**Path Aliases:**
- No path aliases configured; use relative imports
- Internal crate modules use `crate::` prefix in Rust (e.g., `use crate::{ApiResponse, AppStateExt};`)
- Local module imports use `super::*` in test modules

**Import Patterns:**
- TypeScript: Named imports preferred: `import { useState, useEffect } from "react";`
- Type imports use `type` keyword: `import type { UnlistenFn } from "@tauri-apps/api/event";`
- Rust imports grouped by crate then module: `use crate::{audio, db, system_audio};`

## Error Handling

**Patterns:**
- **Tauri Commands:** Return `Result<ApiResponse<T>, String>` where `ApiResponse` wraps success/data/error
- Rust backend uses `anyhow::Result` for internal functions with `.context()` for error messages
- Map errors to user-friendly strings before returning to frontend
- Use `map_err()` to convert error types with descriptive messages

```rust
// Pattern from codebase
pub async fn get_meeting_command(...) -> Result<ApiResponse<MeetingResponse>, String> {
    let meeting = get_meeting(&state.db, id).await.map_err(|e| {
        log::error!("Database error fetching meeting {}: {}", id, e);
        format!("Database error: {}", e)
    })?;
    // ...
}
```

- TypeScript frontend uses try/catch with `instanceof Error` checks
- Error state stored as `string | null` in component state

```typescript
try {
    // ...
} catch (err) {
    setError(err instanceof Error ? err.message : "Failed to load meetings");
}
```

## Logging

**Framework:** `log` crate in Rust, `console.*` in TypeScript

**Patterns:**
- Rust: Use `log::info!`, `log::warn!`, `log::error!` macros
- Include contextual data in log messages (IDs, operation names)
- Log entry and exit of major operations with timing

```rust
log::info!("Generating summary using Ollama at {} with model {}", ollama_url, model);
log::error!("Mic recording error: {}", e);
```

- TypeScript: Use `console.log` for debug, include section labels for boundaries

```typescript
console.log("Meeting created:", meetingId);
```

## Comments

**When to Comment:**
- Document WHY, not WHAT (code should be self-documenting)
- Document complex algorithms or business rules
- Mark workarounds or intentional deviations
- Use `#[allow(dead_code)]` for intentionally unused items in Rust

**JSDoc/TSDoc:**
- Use JSDoc for React components explaining purpose and props
- Use TSDoc for helper functions explaining parameters and return values

```typescript
/**
 * Catches render errors in a subtree and shows a fallback UI instead of
 * crashing the entire app. Wrap each major view with this component.
 */
export class ErrorBoundary extends Component<Props, State> {
```

- Rust: Use `///` doc comments for public items

```rust
/// Extended app state that includes audio recording
pub struct AppStateExt {
    pub db: sqlx::Pool<sqlx::Sqlite>,
    pub audio_recorder: Mutex<AudioRecorder>,
}
```

## Function Design

**Size:** Functions should do one thing; extract helper functions for complex logic

**Parameters:**
- Use destructured props objects for React components
- Rust functions taking multiple related parameters use structs (e.g., `CreateMeetingInput`)
- Prefer slice references over owned collections for flexibility

**Return Values:**
- Rust: Return `Result<T, E>` for fallible operations
- Use custom response wrappers for API consistency (`ApiResponse<T>`)
- TypeScript: Return unions for state (e.g., `type RecordingState = "idle" | "recording" | ...`)

## Module Design

**Exports:**
- React components: Named export (`export function`) + default export (`export default`)
- Rust modules: Public items marked with `pub`, organized in `mod.rs` files
- Commands organized by domain in `commands/` subdirectory

**Barrel Files:**
- Rust: `commands/mod.rs` re-exports all command modules
- No barrel files for TypeScript; import components directly

```rust
// commands/mod.rs
pub mod audio;
pub mod llm;
pub mod meetings;
pub mod settings;
pub mod summaries;
pub mod transcription;
pub mod transcripts;
```

## Architecture Patterns

**Tauri State Pattern:**
- Single `AppStateExt` struct holds all shared state
- Database pool and audio recorder in Mutex for thread safety
- State passed via `tauri::State<'_, AppStateExt>` in commands

**Audio Thread Pattern:**
- `cpal::Stream` is not `Send`/`Sync` — use dedicated audio thread
- Message passing via `std::sync::mpsc` channels for control
- Never store stream in Tauri state

**API Response Pattern:**
- Consistent wrapper struct for all Tauri command responses

```rust
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}
```

**Frontend State Pattern:**
- Loading/error/data pattern for async data fetching
- Refs for cleanup functions (unlisten, intervals)
- `useCallback` for functions passed to child components or effects

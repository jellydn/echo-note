# Code Conventions

## TypeScript / React

### Naming
- Components: `PascalCase` named exports (e.g., `RecordView`, `SettingsView`)
- Hooks: `camelCase` starting with `use` (e.g., `useRecording`)
- Files: Match primary export name (component = `PascalCase.tsx`, utilities = `camelCase.ts`)

### Imports
- Order: Tauri APIs → React hooks → local components → relative siblings
- Example:
  ```ts
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { useState, useEffect, useCallback } from "react";
  import RecordView from "./components/RecordView";
  ```

### Types
- Interfaces defined at the top of each file for props and API responses
- `ApiResponse<T>` generic used for all Tauri command return values
- `strict: true` in tsconfig — no implicit any, unused locals/params are errors

### Formatting (Biome)
- Tab indentation
- 100 character line width
- Double quotes for strings
- Semicolons always

### Error Handling
- Frontend uses try/catch with typed error messages
- User-facing errors shown as toast notifications or inline messages
- Tauri `invoke()` calls wrapped in try/catch blocks

## Rust

### Naming
- Functions and variables: `snake_case`
- Types, structs, enums, traits: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case` (directory-based with `mod.rs`)

### Error Handling
- `anyhow::Result<T>` with `.context("descriptive message")` for propagation
- Tauri commands return `Result<ApiResponse<T>, String>` — never panic across FFI boundary
- Use `?` operator for propagation; convert to string at command boundary
- Logging before returning errors: `log::warn!()`, `log::info!()`, `log::debug!()`

### Tauri Command Pattern
```rust
#[tauri::command]
async fn my_command(
    state: State<'_, AppStateExt>,
    param: String,
) -> Result<ApiResponse<MyType>, String> {
    let result = do_work(&param)
        .map_err(|e| format!("Failed to do work: {}", e))?;
    Ok(ApiResponse::success(result))
}
```

### Response Wrapper
- `ApiResponse<T>` struct with `success(data)` and `error(message)` constructors
- All Tauri commands use this wrapper for consistent frontend handling

### Module Organization
- Each subsystem in its own directory: `audio/`, `db/`, `llm/`, `whisper/`, `system_audio/`
- All Tauri command handlers centralized in `lib.rs`
- Derived traits on public structs: `#[derive(Debug, Serialize, Deserialize)]`

## Shared Patterns

### IPC (Frontend ↔ Backend)
- Frontend: `invoke("command_name", { param })` from `@tauri-apps/api/core`
- Backend: `#[tauri::command]` async functions in `lib.rs`
- Events: `listen("event-name", handler)` for streaming/progress updates

### Data Types
- Request structs: `Deserialize` on Rust side
- Response structs: `Serialize` on Rust side + matching TypeScript interfaces on frontend
- Dates: ISO 8601 strings (`chrono::DateTime` serialized as strings)

### Logging
- Rust: `log::info!()`, `log::warn!()`, `log::debug!()` with contextual messages
- Frontend: `console.log/error` for dev debugging

# Testing Patterns

**Analysis Date:** 2026-04-06

## Test Framework

**Runner:**
- Rust: Built-in `cargo test` (no additional framework)
- TypeScript: None configured yet (noted in AGENTS.md: "No frontend tests exist yet")

**Assertion Library:**
- Rust: Standard `assert!`, `assert_eq!`

**Run Commands:**
```bash
cargo test <test_name>     # Run specific Rust test
cargo test                 # Run all Rust tests
```

## Test File Organization

**Location:**
- Rust: Tests embedded in source files using `#[cfg(test)]` modules
- Pattern: Tests at bottom of `mod.rs` files

**Naming:**
- Rust: `test_*` prefix (e.g., `test_get_model_filename`)

**Structure:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        assert_eq!(...);
    }
}
```

## Existing Tests

**`src-tauri/src/whisper/mod.rs` (lines 358-377):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_model_filename() {
        assert_eq!(get_model_filename("tiny").unwrap(), "ggml-tiny.bin");
        assert_eq!(get_model_filename("small").unwrap(), "ggml-small.bin");
        assert!(get_model_filename("invalid").is_err());
    }

    #[test]
    fn test_default_model() {
        assert_eq!(DEFAULT_MODEL_SIZE, "small");
    }

    #[test]
    fn test_resample_audio() {
        let input = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let output = resample_audio(&input, 16000, 8000);
        assert_eq!(output.len(), 3);
    }
}
```

**`src-tauri/src/system_audio/mod.rs` (lines 112-121):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_string_value() {
        let line = r#""_name" : "MacBook Pro Speakers""#;
        assert_eq!(
            extract_json_string_value(line),
            Some("MacBook Pro Speakers".to_string())
        );
    }
}
```

## Test Patterns

**Setup:**
- No complex setup/teardown currently
- Tests use simple inputs/outputs

**Assertion Pattern:**
- Standard equality and error checking
- `assert_eq!()` for values
- `assert!()` for boolean conditions
- `.is_err()` for error cases

## Mocking

**Framework:** None currently

**What to Mock (when added):**
- Database queries (use in-memory SQLite)
- HTTP requests for model downloads
- Audio devices (cpal)
- File system operations

**Current Approach:**
- Pure functions tested directly (e.g., `resample_audio`, `get_model_filename`)
- I/O-dependent code not tested (audio recording, transcription)

## Test Coverage Gaps

**Untested Areas:**
- `audio/mod.rs`: Audio recording, mixing, WAV writing (cpal dependency)
- `db/mod.rs`: All CRUD operations (SQLite dependency)
- `lib.rs`: All Tauri commands (requires app context)
- Frontend: No tests at all

**High Priority Gaps:**
1. Database CRUD with test database
2. Audio mixing logic (can test without hardware)
3. API response formatting
4. Settings default initialization

**Risk:**
- Audio recording bugs could only be caught manually
- Database query errors only surface at runtime
- Refactoring Tauri commands risky without tests

## Recommended Testing Strategy

**Unit Tests (Priority):**
- Audio processing: `mix_audio_streams`, `resample_audio` (more coverage)
- Database: Use `sqlx::test` with temporary SQLite
- Utilities: JSON parsing, file path handling

**Integration Tests:**
- Tauri command testing via `tauri::test` utilities
- End-to-end via Playwright (frontend)

**Test Data:**
- Create `src-tauri/src/test_fixtures.rs` for sample data
- Factory functions: `create_test_meeting()`, `create_test_transcript()`

---

*Testing analysis: 2026-04-06*

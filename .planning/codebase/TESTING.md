# Testing

## Framework

### Frontend
- **No automated test framework installed** — no jest, vitest, or testing-library in `package.json`
- TypeScript type checking (`bun run typecheck`) serves as static correctness validation
- Biome (`bun run lint`) enforces code quality rules

### Backend (Rust)
- Built-in Rust test framework (`#[test]`, `#[cfg(test)]`)
- Run with: `cargo test` (from `src-tauri/` directory or root)

## Structure

### Test Location
- Rust unit tests co-located in source files within `#[cfg(test)] mod tests { }` blocks
- No dedicated test directories
- No integration test directory (`tests/`) found

### Test File Naming
- Rust: inline in `mod.rs` files — no separate test files

## Frontend Testing
- **Currently: No automated tests**
- Manual testing via `cargo tauri dev` + app interaction
- TypeScript strict mode catches type-level errors at compile time

## Backend Testing

### Pattern
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Arrange, Act, Assert
    }
}
```

### Tests Found
- `whisper/mod.rs`:
  - `test_get_model_filename()` — validates model filename generation
  - `test_default_model()` — validates default model selection
  - `test_resample_audio()` — validates audio resampling logic
- `llm/mod.rs`:
  - `test_build_summary_prompt()` — validates prompt construction
  - `test_parse_summary_response()` — validates LLM response parsing
- `system_audio/mod.rs`:
  - `test_extract_json_string_value()` — validates JSON extraction utility

### Focus
Unit tests for pure utility functions (parsing, validation, audio math). No database, command, or integration tests exist.

## Coverage

### What Is Tested
- Whisper model name/filename utilities
- Audio resampling math
- LLM prompt building
- LLM response parsing
- JSON string extraction

### Gaps
- No database layer tests
- No Tauri command-level tests
- No audio recording/playback tests
- No frontend component tests
- No end-to-end (E2E) tests
- No error path testing
- ~50 lines of tests across ~3,317 lines of Rust code (~1.5% coverage)

## Running Tests
```bash
# Rust unit tests
cargo test                        # all tests
cargo test <test_name>            # specific test

# TypeScript validation
bun run typecheck                 # tsc type check

# Linting
bun run lint                      # Biome check
cargo clippy                      # Rust lints
cargo fmt --check                 # Rust format check

# All checks at once
just check                        # runs all above
```

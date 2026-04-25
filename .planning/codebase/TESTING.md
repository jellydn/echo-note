# Testing Patterns

**Analysis Date:** 2026-04-25

## Test Framework

**Runner:**
- Rust: Built-in `cargo test` (no external test runner)
- TypeScript: No test framework installed (no tests present for frontend)
- Config: Tests embedded in source files under `#[cfg(test)]` modules

**Assertion Library:**
- Rust: Standard library assertions (`assert!`, `assert_eq!`, `assert!().is_err()`)

**Run Commands:**
```bash
just test-rs              # Run all Rust tests
cargo test --manifest-path src-tauri/Cargo.toml  # Direct cargo command
```

## Test File Organization

**Location:**
- Rust: Tests are co-located in source files under `#[cfg(test)] mod tests`
- No separate `tests/` directory for integration tests
- Files with tests: `whisper/mod.rs`, `llm/mod.rs`, `system_audio/mod.rs`

**Naming:**
- Test functions: `test_{descriptive_name}` (e.g., `test_get_model_filename`, `test_parse_summary_response`)

**Structure:**
```
src-tauri/src/
├── whisper/mod.rs       # Contains #[cfg(test)] mod tests
├── llm/mod.rs          # Contains #[cfg(test)] mod tests
├── system_audio/mod.rs # Contains #[cfg(test)] mod tests
└── ... (other modules have no tests)
```

## Test Structure

**Suite Organization:**
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
}
```

**Patterns:**
- No setup/teardown functions; tests are self-contained
- Each test verifies one concept with multiple assertions
- Use `unwrap()` for expected success cases, `is_err()` for expected failures
- Tests use real implementations (not mocked)

## Mocking

**Framework:** None (no mocking library used)

**Patterns:**
- Tests call actual functions with test inputs
- No external dependencies mocked in existing tests
- Functions under test are pure or have minimal side effects

**Example from codebase:**
```rust
#[test]
fn test_resample_audio() {
    let input = vec![0.0, 0.5, 1.0, 0.5, 0.0];
    let output = resample_audio(&input, 16000, 8000);
    assert_eq!(output.len(), 3); // Approximately half the size
}
```

**What to Mock:**
- Not applicable (no mocking pattern established)

**What NOT to Mock:**
- Current tests exercise actual logic without mocks
- Database calls would need mocking if tested
- External API calls (Ollama, Whisper downloads) not unit tested

## Fixtures and Factories

**Test Data:**
- Hardcoded test data in test functions
- No shared fixtures or factory functions

```rust
#[test]
fn test_parse_summary_response() {
    let response = r#"KEY POINTS:
- Discussed project timeline
- Reviewed budget

DECISIONS:
- Approved Q1 plan

ACTION ITEMS:
- Alice: Prepare report
- Bob: Schedule follow-up"#;

    let summary = parse_summary_response(response);
    assert!(summary.key_points.contains("Discussed project timeline"));
    // ...
}
```

**Location:**
- Test data defined inline within each test function
- No external fixture files or test data directories

## Coverage

**Requirements:** None enforced

**View Coverage:**
```bash
# Not configured - no coverage tooling installed
# Could add with cargo-tarpaulin or similar
```

## Test Types

**Unit Tests:**
- Scope: Pure functions and simple logic
- Approach: Test input/output relationships
- Present in: `whisper/mod.rs` (3 tests), `llm/mod.rs` (6 tests), `system_audio/mod.rs` (1 test)

**Integration Tests:**
- Scope: Not currently implemented
- Database operations tested manually or via application
- No automated integration test suite

**E2E Tests:**
- Not used
- Application tested manually through UI

## Common Patterns

**Async Testing:**
- Not present in current test suite
- Async functions tested via application, not unit tests
- Would use `tokio::test` attribute for async tests

**Error Testing:**
```rust
#[test]
fn test_get_model_filename() {
    // Success case
    assert_eq!(get_model_filename("tiny").unwrap(), "ggml-tiny.bin");
    // Error case
    assert!(get_model_filename("invalid").is_err());
}
```

**String Parsing Tests:**
```rust
#[test]
fn test_extract_section() {
    let text = "KEY POINTS:\n- point 1\n\nDECISIONS:\n- decision 1";
    let section = extract_section(text, "KEY POINTS:", Some("DECISIONS:"));
    assert_eq!(section.unwrap().trim(), "- point 1");
}
```

## Test Gaps

**Areas Without Tests:**
- Frontend TypeScript/React code (no test framework installed)
- Database operations (CRUD in `db/mod.rs`)
- Audio recording functionality (`audio/mod.rs`)
- Tauri commands (`commands/*.rs`)
- API integration (Ollama, Whisper downloads)
- Error handling paths

**Testing Debt:**
- No continuous integration configured for automated testing
- No coverage reporting
- Manual testing through application UI

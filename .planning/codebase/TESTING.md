# Testing Patterns

**Analysis Date:** 2026-05-06

EchoNote splits testing across **two stacks**: Vitest + Testing Library for the React frontend, and Rust's built-in `#[test]` harness for the Tauri backend.

## Test Framework

**Runner (Frontend):**
- **Vitest 3.2.x** with the React plugin (`vitest`, `@vitejs/plugin-react` in `package.json`).
- Config: `vitest.config.ts`
  - `globals: true` (no need to import `describe`/`it`).
  - `environment: "jsdom"` (DOM available; `jsdom` 26.x).
  - `setupFiles: ["./src/test/setup.ts"]`.
  - Test glob: `src/**/*.{test,spec}.{ts,tsx}`.

**Runner (Backend):**
- Rust's built-in test harness via `cargo test` (no extra crates beyond stdlib).
- No `#[tokio::test]` usage — all current Rust tests are synchronous, pure-logic checks.

**Assertion Libraries:**
- Frontend: Vitest's `expect` + **`@testing-library/jest-dom`** matchers (`toBeInTheDocument`, etc.) — imported once in `src/test/setup.ts`.
- Frontend interaction: **`@testing-library/react`** v16 + **`@testing-library/user-event`** v14.
- Rust: stdlib `assert!`, `assert_eq!`, `Result::is_err()`.

**Run Commands:**
```bash
# Frontend (defined in package.json scripts)
bun run test           # Vitest watch mode
bun run test:run       # Vitest single run (CI)

# Backend (defined in justfile)
just test-rs           # cargo test --manifest-path src-tauri/Cargo.toml
just test-rs <name>    # filter to a single test, e.g. `just test-rs test_resample_audio`

# Coverage (frontend)
bunx vitest run --coverage   # uses @vitest/coverage-v8 (devDependency)
```

## Test File Organization

**Location:**
- **Frontend tests are separated** into a `__tests__/` sibling folder next to the implementation:
  - `src/components/RecordView.tsx` ↔ `src/components/__tests__/RecordView.test.tsx`
  - `src/components/ErrorBoundary.tsx` ↔ `src/components/__tests__/ErrorBoundary.test.tsx`
  - Same pattern for `HistoryView`, `MeetingDetailView`, `SettingsView`.
- **Rust tests are co-located** in the same file as the code under test, inside a `#[cfg(test)] mod tests { ... }` block:
  - `src-tauri/src/llm/mod.rs` (5 tests at the bottom of the module)
  - `src-tauri/src/whisper/mod.rs` (3 tests)
  - `src-tauri/src/system_audio/mod.rs` (1 test)

**Naming:**
- Frontend: `<ComponentName>.test.tsx`.
- Rust: `fn test_<thing_under_test>()` (e.g. `test_extract_json_string_value`, `test_build_summary_prompt`, `test_resample_audio`).

**Structure:**
```
src/
  components/
    RecordView.tsx
    ErrorBoundary.tsx
    __tests__/
      RecordView.test.tsx
      ErrorBoundary.test.tsx
      HistoryView.test.tsx
      MeetingDetailView.test.tsx
      SettingsView.test.tsx
  test/
    setup.ts                  # global Vitest setup (mocks, jest-dom)

src-tauri/
  src/
    llm/mod.rs                # impl + #[cfg(test)] mod tests at bottom
    whisper/mod.rs
    system_audio/mod.rs
```

## Test Structure

**Suite Organization (Frontend, `src/components/__tests__/RecordView.test.tsx`):**
```typescript
import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RecordView } from "../RecordView";

describe("RecordView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("starts recording when button is clicked", async () => {
    vi.mocked(invoke).mockResolvedValue({ success: true, data: true, error: null });
    render(<RecordView />);
    await userEvent.click(screen.getByRole("button", { name: /start recording/i }));
    await waitFor(() => {
      expect(screen.getByText("Recording in progress...")).toBeInTheDocument();
    });
  });
});
```

**Suite Organization (Rust, `src-tauri/src/llm/mod.rs`):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_summary_prompt() {
        let prompt = build_summary_prompt("Test transcript");
        assert!(prompt.contains("KEY POINTS"));
        assert!(prompt.contains("Test transcript"));
    }
}
```

**Patterns:**
- **Setup:** Frontend uses `beforeEach(() => vi.clearAllMocks())` per suite, in addition to the global `beforeEach` reset in `src/test/setup.ts`. Rust tests need no setup (pure functions).
- **Teardown:** When a test mutates a global (e.g. `console.error` in `src/components/__tests__/ErrorBoundary.test.tsx`), restore it in `afterEach`.
- **Assertion:** Prefer role-based queries (`screen.getByRole("button", { name: /start recording/i })`) over `getByText` for interactive elements; use `getByText` for status / non-interactive text. Use `waitFor` to await async state transitions.
- **Naming:** `it("does X", ...)` describes user-observable behavior, not implementation (`"shows idle state initially"`, `"starts recording when button is clicked"`, `"stops recording and shows title modal"`).

## Mocking

**Framework:** Vitest's built-in `vi.mock` / `vi.fn` / `vi.mocked`.

**Patterns:**

Global Tauri-API mock in `src/test/setup.ts` — applied to **every** frontend test:
```typescript
import "@testing-library/jest-dom";
import { beforeEach, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
  emit: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
});
```

Per-test command-aware mock (from `src/components/__tests__/RecordView.test.tsx`):
```typescript
vi.mocked(invoke).mockImplementation(async (command: string) => {
  if (command === "start_recording_command") {
    return { success: true, data: true, error: null };
  }
  if (command === "stop_recording_command") {
    return {
      success: true,
      data: { file_path: "/path/to/recording.wav", duration_seconds: 60, used_system_audio: true },
      error: null,
    };
  }
  return { success: false, data: null, error: "Unknown command" };
});
```

**What to Mock:**
- **All `@tauri-apps/api/*` imports** — the bridge to Rust is unavailable in `jsdom`, and tests must stay deterministic.
- Console noise from expected-error code paths: `console.error = vi.fn()` in `src/components/__tests__/ErrorBoundary.test.tsx`.
- Always shape mock responses to match the real `ApiResponse<T>` envelope (`{ success, data, error }`) defined in `src-tauri/src/lib.rs`.

**What NOT to Mock:**
- React itself, Testing Library queries, or component internals — drive components via real DOM events (`userEvent.click`, `fireEvent`).
- Pure helpers under test in Rust (the unit tests in `src-tauri/src/whisper/mod.rs`, `llm/mod.rs`, `system_audio/mod.rs` exercise real implementations).
- The SQLite database — currently there are **no DB-integration tests**; commands that touch `state.db` are not covered by automated tests today.

## Fixtures and Factories

**Test Data:**
- No shared factories or fixture files exist. Each test inlines the minimum payload it needs:
  ```typescript
  // src/components/__tests__/RecordView.test.tsx
  return {
    success: true,
    data: { file_path: "/path/to/recording.wav", duration_seconds: 60, used_system_audio: true },
    error: null,
  };
  ```
- Rust tests inline literal inputs (`"Test transcript"`, `vec![0.0, 0.5, 1.0, 0.5, 0.0]` in `src-tauri/src/whisper/mod.rs::test_resample_audio`).

**Location:**
- N/A — fixtures are inlined per test. If duplication grows, the convention should be a `src/test/fixtures/` directory, mirrored by Rust constants in a `mod tests` block.

## Coverage

**Requirements:** **No threshold enforced** — coverage is opt-in.

Coverage config (`vitest.config.ts`):
- Provider: `v8` (`@vitest/coverage-v8` is a devDependency).
- Reporters: `text`, `json`, `html`.
- `include: ["src/components/**/*.tsx"]` — only components are scoped for coverage.
- `exclude: ["src/**/*.d.ts", "src/test/**/*"]`.

**View Coverage:**
```bash
bunx vitest run --coverage     # writes coverage/ with text + html report
```

Rust has no coverage tooling configured (no `cargo-tarpaulin` / `llvm-cov` in `src-tauri/Cargo.toml` or `justfile`).

## Test Types

**Unit Tests:**
- **Rust:** Pure-function unit tests for parsing, prompt-building, audio resampling, and JSON-string extraction (`src-tauri/src/llm/mod.rs::test_parse_summary_response`, `src-tauri/src/whisper/mod.rs::test_resample_audio`, `src-tauri/src/system_audio/mod.rs::test_extract_json_string_value`). They never touch I/O, the DB, or hardware.
- **Frontend:** Component-level unit tests for stateless logic — e.g. `src/components/__tests__/ErrorBoundary.test.tsx` verifies fallback rendering, the `section` prop label, and the "try again" reset.

**Integration Tests:**
- **Frontend:** View-level integration in `src/components/__tests__/RecordView.test.tsx` exercises multi-step flows (idle → recording → stop → save modal → processing), driving real React state transitions while mocking only the Tauri bridge. This is the closest thing to an integration test in the project.
- **Rust:** **None.** No `tests/` directory under `src-tauri/`, no `#[tokio::test]` end-to-end runs against an in-memory SQLite, and no Tauri command-level harness.

**E2E Tests:**
- **Not used.** No Playwright, WebdriverIO, or `tauri-driver` setup; the app is verified manually via `cargo tauri dev` (see `dev` recipe in `justfile`).

## Common Patterns

**Async Testing:**
```typescript
// src/components/__tests__/RecordView.test.tsx
await userEvent.click(screen.getByRole("button", { name: /start recording/i }));
await waitFor(() => {
  expect(screen.getByText("Recording in progress...")).toBeInTheDocument();
});
```
- Always `await userEvent.*` (v14 returns promises).
- Wrap state-dependent assertions in `waitFor(() => expect(...))`; never `setTimeout`.

**Error Testing:**
```typescript
// src/components/__tests__/ErrorBoundary.test.tsx
function ThrowError({ shouldThrow }: { shouldThrow: boolean }) {
  if (shouldThrow) throw new Error("Test error");
  return <div>No error</div>;
}

beforeEach(() => { console.error = vi.fn(); });   // suppress React's expected error log
afterEach(() => { console.error = originalConsoleError; });

it("shows fallback UI when child throws error", () => {
  render(
    <ErrorBoundary>
      <ThrowError shouldThrow={true} />
    </ErrorBoundary>,
  );
  expect(screen.getByText("Something went wrong")).toBeInTheDocument();
});
```

```rust
// src-tauri/src/whisper/mod.rs
#[test]
fn test_get_model_filename() {
    assert_eq!(get_model_filename("tiny").unwrap(), "ggml-tiny.bin");
    assert!(get_model_filename("invalid").is_err());   // assert error path
}
```

## Gaps & Recommendations

- No tests cover Tauri commands themselves (`src-tauri/src/commands/*.rs`) — adding `#[tokio::test]` integration tests with an in-memory SQLite (`sqlite::memory:`) would close a major gap.
- No frontend coverage threshold is enforced; CI does not block on `vitest run --coverage`.
- Consider adding a shared `src/test/factories.ts` once the inline-payload duplication exceeds 3 call-sites.

---

*Testing analysis: 2026-05-06*

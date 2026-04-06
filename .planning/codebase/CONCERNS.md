# Technical Concerns

## Known Issues & TODOs
- No explicit TODO/FIXME/HACK comments found in the source code (well-disciplined)
- Issues exist nonetheless — identified through code analysis below

## Security Concerns

### High Priority
- **API Key Plaintext Storage**: API keys stored unencrypted in SQLite `settings` table — should use macOS Keychain
- **Transcript Privacy**: Meeting transcriptions not encrypted at rest in the database
- **SSRF Risk**: User-provided API endpoints (in settings) not validated for scheme or allowlisted domains
- **No Input Sanitization**: Meeting titles and API endpoints accepted without sanitization

## Error Handling Issues

### Panics in Production Code
6 `unwrap()` / `expect()` calls identified in production paths:
- `src-tauri/src/whisper/mod.rs:283` — path `unwrap()` can panic on invalid UTF-8
- `src-tauri/src/audio/mod.rs:369` — `expect()` on device detection panics if device unavailable
- Multiple `Mutex::lock().unwrap()` calls throughout the audio module

### Silent Error Loss
- `eprintln!()` used in recording threads instead of proper error propagation
- Errors in background audio threads are swallowed, not surfaced to UI

### Race Conditions
- Recording state cloned without guaranteed synchronization
- Potential state inconsistencies between frontend-perceived state and actual recorder state

## Performance Concerns

### Memory
- Entire audio recordings held in memory (Vec<f32>) before writing to disk — problematic for recordings >30 minutes
- Whisper models loaded entirely into RAM (1–2GB+ for medium/large models); no lazy loading
- Audio resampling done in-memory at transcription time
- Full transcript text sent to LLM without chunking for long meetings

### Processing
- No timeout on external API calls (Ollama / OpenAI) — can hang indefinitely
- No retry mechanism for failed transcriptions or LLM requests

## Fragile Areas

### LLM Response Parsing
- `llm/mod.rs` uses regex-based parsing of LLM output — brittle and model-dependent
- Any change in Ollama/OpenAI response format can break summary extraction

### Audio Thread Architecture
- `audio/mod.rs` (568 lines) manages multiple responsibilities: device enumeration, recording, mixing, WAV writing
- `cpal::Stream` not `Send`/`Sync` — workaround via dedicated thread adds complexity and potential deadlocks

### Large Files (Complexity Risk)
- `src-tauri/src/lib.rs`: 1,087 lines — all 40 Tauri commands in one file
- `src/components/SettingsView.tsx`: 680 lines
- `src/components/RecordView.tsx`: 566 lines
- `src-tauri/src/audio/mod.rs`: 568 lines

## Missing Features / Gaps

### Robustness
- No retry on failed transcriptions or API calls
- No cleanup of incomplete/failed recordings
- No timeout on HTTP requests (`reqwest` without `.timeout()`)
- No database migration versioning system — schema managed inline

### Frontend
- No React error boundaries — a component crash brings down the entire app
- No loading states for some async operations

### Operational
- Whisper transcription language hardcoded to English (no language detection)
- No rate limiting on API calls
- No structured/persistent logging (logs not written to file)
- No auto-update mechanism
- No crash reporting

## Technical Debt

### Test Coverage
- ~50 lines of tests across ~3,317 lines of Rust code (~1.5% coverage)
- Zero frontend tests
- No integration or E2E tests

### Architecture
- `lib.rs` acts as a monolithic command handler — all 40 commands in one file
- `SettingsView.tsx` and `RecordView.tsx` are oversized and handle too many concerns
- No React global state (no Context or Zustand) — some prop/state duplication across views

### Compliance / Privacy
- No user consent tracking for stored audio/transcript data
- No data export or deletion mechanism (GDPR gap)
- Model downloads fixed to app data directory (can't configure external drive)

## Dependencies Concerns
- All major dependencies appear actively maintained (Tauri v2, whisper-rs 0.13, reqwest 0.12, sqlx 0.8)
- `whisper-rs` may lag behind upstream whisper.cpp releases
- `cpal` has platform-specific maintenance variability on non-macOS platforms

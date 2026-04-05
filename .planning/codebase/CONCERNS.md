# Codebase Concerns

**Analysis Date:** 2026-04-06

## Tech Debt

**Audio Thread Complexity:**
- Issue: cpal `Stream` is not `Send`/`Sync` requiring complex thread management
- Files: `src-tauri/src/audio/mod.rs` (lines 63-277)
- Impact: Risk of thread panics, hard to test, potential race conditions
- Fix approach: Abstract into actor pattern or use message passing exclusively

**Large Command Functions:**
- Issue: `transcribe_audio_command` and `transcribe_audio` are ~150+ lines
- Files: `src-tauri/src/lib.rs`, `src-tauri/src/whisper/mod.rs`
- Impact: Hard to read, test, and maintain
- Fix approach: Extract into smaller functions with single responsibilities

**Monolithic lib.rs:**
- Issue: All Tauri commands in single ~600 line file
- File: `src-tauri/src/lib.rs`
- Impact: Merge conflicts, hard to navigate
- Fix approach: Split into command modules (e.g., `commands/meetings.rs`, `commands/audio.rs`)

**Minimal Frontend:**
- Issue: `App.tsx` has placeholder views only
- File: `src/App.tsx`
- Impact: Not a functional app yet, UI needs full implementation
- Fix approach: Implement views per PRD user stories

## Known Issues

**Audio Thread Panic Recovery:**
- Symptoms: Recording thread panics leave app in bad state
- Files: `src-tauri/src/audio/mod.rs` (lines 205-224)
- Trigger: Device disconnection during recording
- Workaround: Restart app, no auto-recovery

**TODO/FIXME Comments:**
- None found in current codebase (good sign)

## Security Considerations

**Unencrypted API Keys:**
- Risk: API keys stored as plain text in SQLite
- Files: `src-tauri/src/db/mod.rs` (settings table)
- Current mitigation: None - local app only
- Recommendations: Use macOS Keychain or encrypted storage

**CSP Disabled:**
- Risk: Content Security Policy is `null` (disabled)
- File: `src-tauri/tauri.conf.json` (line 22)
- Current mitigation: Local app, no web content
- Recommendations: Set appropriate CSP for production

**File Path Traversal:**
- Risk: User-controlled paths could access arbitrary files
- Files: `src-tauri/src/audio/mod.rs` (recording path generation)
- Current mitigation: Timestamps used in filenames
- Recommendations: Validate and sanitize all paths

**Audio Data in Memory:**
- Risk: Raw audio data held in memory during recording
- Files: `src-tauri/src/audio/mod.rs` (lines 91-92)
- Current mitigation: Memory cleared after save
- Recommendations: Stream to disk for long recordings

## Performance Bottlenecks

**Transcription Blocking:**
- Problem: Whisper transcription runs on blocking thread but still blocks Tauri
- Files: `src-tauri/src/lib.rs` (lines 476-490)
- Cause: `tokio::task::spawn_blocking` used but progress events still synchronous
- Improvement path: Use async channels for progress, consider process isolation

**Audio Buffer Growth:**
- Problem: Audio data grows unbounded in `Vec<f32>` during recording
- Files: `src-tauri/src/audio/mod.rs` (line 92)
- Cause: All audio held in memory until recording stops
- Improvement path: Stream to temp file, or use circular buffer with disk offload

**Model Loading:**
- Problem: Whisper model loaded into memory on each transcription
- Files: `src-tauri/src/whisper/mod.rs` (lines 290-296)
- Cause: No model caching between transcriptions
- Improvement path: Cache `WhisperContext` in app state

## Fragile Areas

**Device Matching Logic:**
- Files: `src-tauri/src/audio/mod.rs` (lines 314-337)
- Why fragile: Partial string matching for BlackHole/microphone
- Safe modification: Add explicit device ID storage in settings
- Test coverage: No tests for device selection

**JSON Parsing in System Audio:**
- Files: `src-tauri/src/system_audio/mod.rs` (lines 62-78)
- Why fragile: Manual string parsing instead of serde_json
- Safe modification: Use proper JSON parsing with structs
- Test coverage: Single basic test only

**Sample Rate Conversion:**
- Files: `src-tauri/src/whisper/mod.rs` (lines 362-375)
- Why fragile: Linear interpolation resampling is low quality
- Safe modification: Use proper resampling library (e.g., `rubato`)
- Test coverage: Single basic test

## Scaling Limits

**Audio Recording:**
- Current capacity: Memory-limited (RAM holds all audio)
- Limit: ~30-60 minutes at 16kHz before memory pressure
- Scaling path: Stream to disk, implement chunked recording

**Whisper Model Size:**
- Current capacity: medium model (~769MB)
- Limit: Larger models need more RAM, slower inference
- Scaling path: GPU acceleration, model quantization

**Database:**
- Current capacity: Single SQLite file
- Limit: Concurrent writes, large meeting history
- Scaling path: Connection pooling already implemented

## Dependencies at Risk

**whisper-rs:**
- Risk: Bindings to whisper.cpp C++ library, compilation complexity
- Impact: Build failures, platform-specific issues
- Migration plan: Alternative Rust-native ASR (none mature yet)

**cpal:**
- Risk: Platform-specific audio backend issues (CoreAudio on macOS)
- Impact: Recording failures on some systems
- Migration plan: Platform-specific backends (e.g., `coreaudio-rs` directly)

**BlackHole Driver:**
- Risk: Third-party kernel extension dependency
- Impact: macOS security changes may block installation
- Migration plan: ScreenCaptureKit API (macOS 13+) for system audio

## Missing Critical Features

**LLM Summary Generation:**
- Problem: Ollama integration referenced but not implemented
- Files: `src-tauri/src/lib.rs` (imports only, no commands)
- Blocks: Cannot generate meeting summaries
- Priority: High (core feature per PRD)

**Frontend UI Implementation:**
- Problem: All views are placeholders
- Files: `src/App.tsx` (lines 10-35)
- Blocks: App not usable
- Priority: High

**Error Handling UX:**
- Problem: No frontend error display, toast notifications
- Blocks: Users can't diagnose failures
- Priority: Medium

## Test Coverage Gaps

**Untested Critical Paths:**
1. Audio recording start/stop
2. Database CRUD operations
3. Tauri command handlers
4. File I/O operations
5. Model download with progress

**Risk Assessment:**
- High: Audio recording (hardware dependent, hard to reproduce issues)
- Medium: Database queries (SQLx compile-time checking helps)
- Medium: Tauri commands (integration tests needed)

---

*Concerns audit: 2026-04-06*

# Codebase Concerns

**Analysis Date:** 2025-04-25

## Tech Debt

**Audio Thread Pattern:**
- Issue: Using `std::sync::Mutex` poison recovery via `unwrap_or_else(|e| e.into_inner())` in multiple places - this is a workaround for potential lock poisoning
- Files: `src-tauri/src/audio/mod.rs` (lines 69, 209, 277, 437)
- Impact: If a thread panics while holding a lock, the mutex becomes "poisoned". The current recovery approach may mask underlying issues
- Fix approach: Consider using `parking_lot::Mutex` which doesn't have poisoning, or refactor to use message passing/channels exclusively

**Hardcoded Model Configuration:**
- Issue: API model is hardcoded to `gpt-4o-mini` in the LLM command handler
- Files: `src-tauri/src/commands/llm.rs` (line 76)
- Impact: Users cannot configure which API model to use when using cloud providers
- Fix approach: Add a setting for API model selection, similar to how Ollama model is configurable

**Date Format Parsing:**
- Issue: Manual RFC3339 date parsing in command handlers instead of using strongly typed deserialization
- Files: `src-tauri/src/commands/meetings.rs` (lines 46-48)
- Impact: Date parsing errors happen at runtime; frontend/backend coupling on string formats
- Fix approach: Use `chrono::DateTime<chrono::Utc>` directly in the request struct with proper serde annotations

## Known Bugs

**Transcription Audio Processing:**
- Symptoms: Stereo audio channel handling in `whisper/mod.rs` has a flawed conversion logic - samples are zeroed for non-first channels then averaged, potentially causing audio quality issues
- Files: `src-tauri/src/whisper/mod.rs` (lines 319-345)
- Trigger: Recording with multi-channel input devices
- Workaround: None identified - audio may have reduced quality for stereo sources

**System Audio Thread Panic Handling:**
- Symptoms: System audio thread panics are logged but silently ignored (mic-only fallback)
- Files: `src-tauri/src/audio/mod.rs` (lines 199-203)
- Trigger: BlackHole device issues or cpal errors during system audio capture
- Workaround: Recording continues with microphone only, but user is not notified of system audio failure

**API Endpoint URL Construction:**
- Symptoms: Fragile string manipulation for API endpoint URLs
- Files: `src-tauri/src/commands/llm.rs` (lines 66-74)
- Trigger: User enters malformed or unexpected API endpoint format
- Workaround: URL normalization logic attempts to handle common cases but edge cases may fail

## Security Considerations

**API Key Storage:**
- Risk: API keys stored in SQLite database without encryption
- Files: `src-tauri/src/db/mod.rs` (settings table), `src-tauri/src/commands/llm.rs`
- Current mitigation: Keys are stored locally only; app claims "privacy-first" approach
- Recommendations: 
  - Use macOS Keychain or similar OS-level secure storage
  - At minimum, encrypt sensitive settings with a device-specific key
  - Consider marking the key field as "sensitive" in the database schema

**CSP Configuration:**
- Risk: Content Security Policy is set to `null` in Tauri config
- Files: `src-tauri/tauri.conf.json` (line 21)
- Current mitigation: None - effectively disabled
- Recommendations: Define a restrictive CSP appropriate for the local app context

**External Process Execution:**
- Risk: Command injection via `system_audio/mod.rs` osascript execution for Homebrew installation
- Files: `src-tauri/src/system_audio/mod.rs` (lines 201-210)
- Current mitigation: Arguments are hardcoded, not user-input
- Recommendations: Sanitize any future user input that might flow into command execution

## Performance Bottlenecks

**Transcription Blocking:**
- Problem: Whisper transcription runs entirely on the main async runtime via `spawn_blocking`
- Files: `src-tauri/src/commands/transcription.rs` (lines 56-61)
- Cause: Large audio files can take significant time to transcribe, blocking a thread pool thread
- Improvement path: Consider using a dedicated thread pool or process pool for CPU-intensive transcription work

**Audio Buffer Growth:**
- Problem: Audio recording accumulates samples in unbounded `Vec<f32>` buffers in memory
- Files: `src-tauri/src/audio/mod.rs` (lines 88, 119, 480-488)
- Cause: Long recordings could theoretically exhaust memory
- Improvement path: Implement circular buffering or stream-to-disk during recording for very long sessions

**Model Download:**
- Problem: Large model downloads (up to 1.5GB) happen in-memory before writing to disk
- Files: `src-tauri/src/whisper/mod.rs` (lines 147-182)
- Cause: `bytes_stream` is consumed chunk by chunk, which is good, but no disk caching during download
- Improvement path: Current implementation is reasonable but verify temp file handling for interrupted downloads

## Fragile Areas

**BlackHole Detection:**
- Files: `src-tauri/src/system_audio/mod.rs`
- Why fragile: Multiple fallback detection methods (system_profiler JSON, line parsing, HAL directory check) indicate platform-dependent reliability issues
- Safe modification: Test on various macOS versions; consider CoreAudio API bindings instead of CLI tools
- Test coverage: Only unit test for JSON parsing; no integration tests for actual device detection

**Audio Device Name Matching:**
- Files: `src-tauri/src/audio/mod.rs` (lines 351-392)
- Why fragile: String-based partial matching for BlackHole and microphone devices may fail with localized device names or different BlackHole variants
- Safe modification: Use stable device IDs where possible; add more robust fuzzy matching
- Test coverage: No tests for device selection logic

**LLM Response Parsing:**
- Files: `src-tauri/src/llm/mod.rs` (lines 272-297)
- Why fragile: Summary section extraction relies on exact string matching for headers ("KEY POINTS:", "DECISIONS:", etc.) with case variations
- Safe modification: Use structured JSON responses from LLMs when available; add more robust parsing with regex or markdown parsing
- Test coverage: Has unit tests for parsing, but limited edge case coverage

**Audio Sample Format Handling:**
- Files: `src-tauri/src/audio/mod.rs` (lines 407-423)
- Why fragile: Only handles F32, I16, U16 sample formats; others cause hard error
- Safe modification: Add more format conversions or better error messaging for unsupported devices
- Test coverage: No tests for sample format conversion

## Scaling Limits

**Database:**
- Current capacity: SQLite with 5 connection pool; no stated limits on meeting count
- Limit: Single-writer limitations of SQLite; concurrent writes may queue
- Scaling path: For high concurrency, migrate to PostgreSQL or implement write queueing

**Audio Recording:**
- Current capacity: Limited by available RAM (buffers held in memory)
- Limit: Long meetings (>2 hours) may consume significant memory before WAV writing
- Scaling path: Implement chunked recording or streaming write to disk

**Transcript Storage:**
- Current capacity: Stored as TEXT in SQLite
- Limit: Very long transcripts may hit SQLite row limits or performance degradation
- Scaling path: Consider compression or chunked storage for large transcripts

## Dependencies at Risk

**whisper-rs:**
- Risk: Bindings to whisper.cpp which evolves rapidly; API may change
- Impact: Transcription may break on updates
- Migration plan: Pin to specific version; monitor whisper.cpp releases

**cpal:**
- Risk: Cross-platform audio is complex; macOS-specific issues may arise
- Impact: Audio recording failures on certain hardware configurations
- Migration plan: Consider platform-specific backends (CoreAudio directly) if issues persist

**BlackHole Driver:**
- Risk: Third-party kernel driver dependency; may break with macOS updates
- Impact: System audio capture completely fails
- Migration plan: Monitor BlackHole project; document manual installation steps; consider alternative virtual audio drivers

**ollama/reqwest:**
- Risk: Local LLM server may not be running; network timeouts
- Impact: Summary generation fails or hangs
- Migration plan: Implement better timeout handling; consider bundled LLM option

## Missing Critical Features

**Real-time Transcription:**
- Problem: Transcription only starts after recording stops
- Blocks: Live captioning or real-time meeting assistance features

**Audio File Cleanup:**
- Problem: Original WAV files are never deleted; storage grows unbounded
- Blocks: Long-term usage without manual file management

**Meeting Export:**
- Problem: No export functionality for transcripts/summaries (Markdown, PDF, etc.)
- Blocks: Sharing meeting notes outside the app

**Search Functionality:**
- Problem: No search across meeting transcripts or summaries
- Blocks: Finding information in past meetings

**Speaker Diarization:**
- Problem: No identification of who spoke when
- Blocks: Multi-speaker meeting analysis

## Test Coverage Gaps

**Integration Tests:**
- What's not tested: End-to-end audio recording flow, database operations with real SQLite, actual Whisper transcription
- Files: All `src-tauri/src/` modules
- Risk: Command handlers may have integration issues not caught by unit tests
- Priority: High

**Error Handling Paths:**
- What's not tested: Error recovery in audio recording, database connection failures, network timeouts during model download
- Files: `src-tauri/src/audio/mod.rs`, `src-tauri/src/db/mod.rs`, `src-tauri/src/whisper/mod.rs`
- Risk: Error paths may panic or leave system in inconsistent state
- Priority: High

**Frontend Component Tests:**
- What's not tested: React components have no test suite
- Files: `src/components/*.tsx`
- Risk: UI regressions not caught before release
- Priority: Medium

**Concurrency Tests:**
- What's not tested: Multi-threaded audio recording, concurrent database access
- Files: `src-tauri/src/audio/mod.rs`
- Risk: Race conditions in audio buffer access
- Priority: Medium

**Cross-Platform Tests:**
- What's not tested: App is macOS-only but structure suggests potential cross-platform support
- Files: `src-tauri/src/system_audio/mod.rs` (macOS-specific)
- Risk: Hardcoded macOS assumptions may break if ported
- Priority: Low (current scope is macOS only)

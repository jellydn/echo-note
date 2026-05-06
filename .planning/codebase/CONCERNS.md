# Codebase Concerns

**Analysis Date:** 2026-05-06

## Tech Debt

**Audio recorder uses shared mutable buffers across recording threads:**
- Issue: `AudioRecorder` keeps `Arc<Mutex<Vec<f32>>>` buffers and accepts poisoned recording-state locks in several paths.
- Files: `src-tauri/src/audio/mod.rs:20`, `src-tauri/src/audio/mod.rs:67`, `src-tauri/src/audio/mod.rs:209`, `src-tauri/src/audio/mod.rs:277`, `src-tauri/src/audio/mod.rs:437`
- Impact: The current pattern works around `cpal::Stream` being thread-bound, but long callbacks and lock poisoning can make failure modes hard to reason about.
- Fix approach: Keep the dedicated audio-thread pattern, but move toward channel-backed sample delivery or stream-to-disk chunks so callbacks do minimal locking.

**Whisper transcription owns the full decoded audio in memory:**
- Issue: Transcription reads the whole WAV into memory before resampling and chunking for Whisper.
- Files: `src-tauri/src/whisper/mod.rs:319`, `src-tauri/src/whisper/mod.rs:349`
- Impact: Long recordings allocate at least one full `Vec<i16>` plus one full `Vec<f32>` before inference begins.
- Fix approach: Decode, downmix, resample, and feed chunks incrementally, or write an intermediate normalized file for bounded-memory processing.

## Known Bugs

**Microphone test only builds an `f32` input stream:**
- Symptoms: `test_microphone` can fail on devices whose default input sample format is not `f32`, even though normal recording supports `F32`, `I16`, and `U16`.
- Files: `src-tauri/src/audio/mod.rs:546`, `src-tauri/src/audio/mod.rs:554`, `src-tauri/src/audio/mod.rs:407`
- Trigger: Selecting an input device whose default `cpal` sample format is `I16` or `U16`.
- Workaround: Use the default device only if it exposes `f32`, or rely on the recording path instead of the mic-test path.

**System-audio capture failures are silent in the UI:**
- Symptoms: BlackHole/system recording errors are logged and recording falls back to mic-only, but the frontend only sees `used_system_audio: false` after stop.
- Files: `src-tauri/src/audio/mod.rs:191`, `src-tauri/src/audio/mod.rs:195`, `src-tauri/src/audio/mod.rs:199`, `src/components/RecordView.tsx:263`
- Trigger: BlackHole device disappears, fails to open, or its recording thread panics during capture.
- Workaround: The saved recording still contains microphone audio; users must notice from the post-stop result or logs.

## Security Considerations

**API keys are stored as plain SQLite settings:**
- Risk: `api_key` is persisted in the app SQLite database like any other setting.
- Files: `src-tauri/src/db/mod.rs:67`, `src-tauri/src/db/mod.rs:460`, `src/components/SettingsView.tsx:653`
- Current mitigation: Data is local to the user's app data directory and cloud summarization is opt-in.
- Recommendations: Store secrets in macOS Keychain or another OS credential store; keep SQLite for non-secret settings only.

**External installer commands rely on shelling out to macOS tools:**
- Risk: BlackHole install flows invoke `open`, `which`, and `osascript` with hardcoded arguments.
- Files: `src-tauri/src/system_audio/mod.rs:170`, `src-tauri/src/system_audio/mod.rs:181`, `src-tauri/src/system_audio/mod.rs:192`
- Current mitigation: Commands do not interpolate user input into the shell script.
- Recommendations: Keep command arguments hardcoded, avoid adding user-controlled interpolation, and prefer opening official install pages over constructing shell commands.

## Performance Bottlenecks

**Recordings are buffered in memory until stop:**
- Problem: Mic and system audio samples are pushed into grow-only `Vec<f32>` buffers and only written to WAV on stop.
- Files: `src-tauri/src/audio/mod.rs:88`, `src-tauri/src/audio/mod.rs:119`, `src-tauri/src/audio/mod.rs:480`
- Cause: The recorder optimizes for simple post-stop mixing rather than streaming writes.
- Improvement path: Add chunked disk writes or periodic flushes, then mix from bounded chunks.

**Whisper model loads per transcription:**
- Problem: `transcribe_audio` constructs a new `WhisperContext` for each transcription.
- Files: `src-tauri/src/whisper/mod.rs:284`, `src-tauri/src/whisper/mod.rs:289`
- Cause: Model lifetime is scoped to a single command call.
- Improvement path: Cache model contexts by selected model size behind a controlled worker or pool, while accounting for memory pressure.

## Fragile Areas

**BlackHole detection is platform and output-format dependent:**
- Files: `src-tauri/src/system_audio/mod.rs:12`, `src-tauri/src/system_audio/mod.rs:36`, `src-tauri/src/system_audio/mod.rs:58`, `src-tauri/src/system_audio/mod.rs:107`
- Why fragile: Detection combines `system_profiler -json`, line parsing, and HAL-directory checks.
- Safe modification: Keep all fallbacks until a CoreAudio-backed detector replaces them; test on multiple macOS versions.
- Test coverage: Only JSON string extraction is unit-tested in `src-tauri/src/system_audio/mod.rs:226`.

**Audio device selection still depends on display names:**
- Files: `src-tauri/src/audio/mod.rs:351`, `src-tauri/src/audio/mod.rs:511`, `src-tauri/src/db/mod.rs:457`
- Why fragile: The app now stores real device names instead of synthetic `device_0` IDs, but CoreAudio display names can still change or collide.
- Safe modification: Preserve `default` and exact-name matching while investigating stable host-specific identifiers.
- Test coverage: No automated tests cover device enumeration or selection fallback.

**LLM response parsing expects textual section headers:**
- Files: `src-tauri/src/llm/mod.rs:288`
- Why fragile: Summary parsing relies on the model returning recognizable `KEY POINTS`, `DECISIONS`, and `ACTION ITEMS` sections.
- Safe modification: Prefer structured JSON output or schema-constrained responses for API providers; keep parser tests for legacy text output.
- Test coverage: Unit tests cover basic parsing, but not broad malformed or localized response shapes.

## Scaling Limits

**SQLite persistence is local single-user storage:**
- Current capacity: One local SQLite database with a five-connection pool.
- Limit: Suitable for a desktop app, but not for shared/team meeting repositories.
- Scaling path: Keep SQLite for local-first storage; add export/sync as a separate architecture decision if collaboration becomes a requirement.

**Transcript and audio retention is unbounded:**
- Current capacity: WAV files live under the app recordings directory and transcripts/summaries remain in SQLite.
- Limit: Long-term use can consume significant disk space.
- Scaling path: Add retention controls, per-meeting delete confirmation that includes audio files, and optional export/archive flows.

## Dependencies at Risk

**BlackHole driver:**
- Risk: Third-party virtual audio dependency can break with macOS updates or missing user installation.
- Impact: System audio capture becomes unavailable; microphone-only recording still works.
- Migration plan: Keep in-app setup guidance current and evaluate native/system-extension alternatives only if BlackHole proves unreliable.

**whisper-rs / whisper.cpp model compatibility:**
- Risk: Inference bindings and downloaded model formats can drift.
- Impact: Transcription fails at model load time or produces degraded output.
- Migration plan: Pin known-good versions, validate downloaded model sizes/checksums, and add a small transcription smoke test fixture.

## Missing Critical Features

**Search and export:**
- Problem: Meetings can be stored and viewed, but there is no full-text search or export path for transcripts/summaries.
- Blocks: Reusing notes outside the app and finding past discussion details quickly.

**First-run BlackHole install verification:**
- Problem: The app exposes status and install actions, but the bundled-installer path remains a product concern until the actual `.pkg` asset and first-launch install flow are verified end to end.
- Blocks: Marking bundled system-audio setup as complete.

## Test Coverage Gaps

**Tauri command integration tests:**
- What's not tested: IPC command handlers across state, settings, database, audio, Whisper, and LLM boundaries.
- Files: `src-tauri/src/commands/*.rs`
- Risk: Frontend and Rust unit tests can pass while command payloads or state wiring regress.
- Priority: High

**Hardware and OS integration tests:**
- What's not tested: Real microphone capture, BlackHole detection, system-audio fallback, and sample-format variants.
- Files: `src-tauri/src/audio/mod.rs`, `src-tauri/src/system_audio/mod.rs`
- Risk: Audio regressions can only be caught manually on affected hardware.
- Priority: High

**Resolved in this update:**
- Fixed stereo/multi-channel WAV downmixing so non-first channels are averaged instead of zeroed before Whisper transcription.
- Added an empty-input guard to `resample_audio` to avoid indexing into an empty sample buffer.
- Files: `src-tauri/src/whisper/mod.rs:319`, `src-tauri/src/whisper/mod.rs:410`, `src-tauri/src/whisper/mod.rs:436`, `src-tauri/src/whisper/mod.rs:474`, `src-tauri/src/whisper/mod.rs:479`

---

*Concerns audit: 2026-05-06*

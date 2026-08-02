# Audio Hardware & macOS Integration Test Matrix

**Issue:** #37 — Add hardware and macOS audio integration coverage.

Audio behaviour depends on real CoreAudio devices, BlackHole installation, and
`cpal` sample formats. Unit tests cannot catch failures that only occur on
specific macOS versions or hardware, so this matrix defines the manual (and
opt-in automated) scenarios every supported release should pass, and records
the expected UI/log outcome for each.

## Diagnostics command

Every scenario starts from a reproducible environment snapshot:

```bash
# Frontend DevTools console:
await invoke("get_audio_diagnostics_command")
```

Response shape:

```json
{
  "success": true,
  "data": {
    "default_device": "MacBook Pro Microphone",
    "devices": [
      {
        "id": "coreaudio-input:MacBook Pro Microphone#0",
        "name": "MacBook Pro Microphone",
        "sample_format": "f32",
        "channels": 1,
        "sample_rate": 48000
      },
      {
        "id": "coreaudio-input:BlackHole 2ch#0",
        "name": "BlackHole 2ch",
        "sample_format": "f32",
        "channels": 2,
        "sample_rate": 48000
      }
    ],
    "blackhole_installed": true,
    "blackhole_device": "BlackHole 2ch"
  }
}
```

Record this payload in the bug/PR when a hardware scenario fails. If
`sample_format` is `null` for a device, that device exposed no default input
config — log that too.

## Supported macOS versions

| macOS | Architecture | Notes |
|-------|--------------|-------|
| Sequoia (15.x) | Apple Silicon (arm64) | Primary dev target |
| Sequoia (15.x) | Intel (x86_64) | Rosetta / native |
| Sonoma (14.x) | Apple Silicon | |
| Ventura (13.x) | Intel | Oldest supported |

## Scenario matrix

Check each scenario, record diagnostics, and confirm the expected UI/log
outcome. Scenarios with an **Opt-in** tag can be automated with a hardware test
runner that gates on the presence of the required device.

### S1. Default microphone capture

| | |
|---|---|
| **Setup** | No non-default device selected; `audio_device` setting = `default`. |
| **Steps** | Record 5s, stop, play back. |
| **Expected** | `start_recording` ok; WAV written; `used_system_audio` = `false` (unless BlackHole active, see S3). |
| **UI warning** | None. |
| **Logs** | `cpal` stream started; no errors. |
| **Opt-in automation** | Yes — requires a working mic; skip if `default_device` is `null`. |

### S2. Non-f32 sample format device

| | |
|---|---|
| **Setup** | Device whose `default_input_config().sample_format()` is `I16`/`U16` (e.g. some USB interfaces). |
| **Steps** | Select device; record; run mic test (`test_microphone_command`). |
| **Expected** | Recording succeeds (recording path supports F32/I16/U16). Mic test succeeds (also handles I16/U16). |
| **UI warning** | None expected. |
| **Logs** | Sample format logged; conversion via `mono_sample_from_frame`. |
| **Opt-in automation** | Yes — gate on a device with `sample_format` ∈ {`i16`, `u16`, `u32`, …} in diagnostics. |
| **Known gap** | `test_microphone` fails on formats outside F32/I16/U16 — see `CONCERNS.md` Known Bugs. |

### S3. BlackHole installed — system audio captured

| | |
|---|---|
| **Setup** | `get_audio_diagnostics_command` reports `blackhole_installed: true`. |
| **Steps** | Play audio in another app, record with system audio enabled, stop. |
| **Expected** | `used_system_audio` = `true`; mixed output audible in WAV. |
| **UI warning** | None. |
| **Logs** | BlackHole device found; system capture thread started. |

### S4. BlackHole missing — mic-only fallback

| | |
|---|---|
| **Setup** | `blackhole_installed: false` (fresh install or driver removed). |
| **Steps** | Record with system audio requested; stop. |
| **Expected** | Recording succeeds; `used_system_audio` = `false`. |
| **UI warning** | UI shows system-audio unavailable hint (per existing behaviour). |
| **Logs** | `system_audio` module logs detection failure and falls back to mic-only. |

### S5. System-audio capture failure mid-recording

| | |
|---|---|
| **Setup** | BlackHole installed, then removed/disabled while recording (or device open fails). |
| **Steps** | Start recording, break the BlackHole device, stop. |
| **Expected** | Recording still saved with mic audio; `used_system_audio` = `false`; `system_audio_error` populated in the stop response. |
| **UI warning** | Post-stop result surfaces `system_audio_error` (see `RecordView.tsx` truncated/error handling). |
| **Logs** | `system-audio capture failed: …` from the recording thread. |
| **Known gap** | Failure was historically silent until the stop response — see `CONCERNS.md` Known Bugs. |

### S6. No input device at all

| | |
|---|---|
| **Setup** | All input devices disabled in System Settings (or headless CI). |
| **Steps** | Click start recording. |
| **Expected** | Graceful error surfaced to UI; no panic. |
| **UI warning** | "No audio input device available" style message. |
| **Logs** | `resolve_input_device` / `default_input_device` error logged. |
| **Opt-in automation** | Yes — CI job with no audio hardware expects this path. |

### S7. Microphone permission denied

| | |
|---|---|
| **Setup** | App removed from Privacy & Security → Microphone. |
| **Steps** | Attempt recording. |
| **Expected** | CoreAudio returns permission error; UI shows actionable message. |
| **UI warning** | Permission prompt / "grant microphone access" message. |
| **Logs** | cpal build-stream error surfaces. |

## Frontend UI mapping

Expected warnings referenced above live in:

- `src/components/RecordView.tsx` — recording state, truncation and
  system-audio error surfacing.
- `src/components/SettingsView.tsx` — device picker, mic test, BlackHole
  install status.

## How to run

1. Install the app (`bun run tauri dev` or a packaged build).
2. Capture diagnostics (`get_audio_diagnostics_command`) before each scenario.
3. Work through S1–S7 on each supported macOS version.
4. For automated runs, gate opt-in scenarios on diagnostics (device presence +
   sample format) so CI skips gracefully.

## Maintenance

Update this matrix whenever a new macOS version is added to support, or when
the `cpal` sample-format handling changes (see `audio/mod.rs`
`sample_format_label`, `collect_device_diagnostics`).

//! Golden-output source of truth for the export mock.
//!
//! The frontend Vitest suite (`src/mocks/__tests__/exportGolden.test.ts`)
//! asserts the mock builders reproduce the *committed* golden outputs in
//! `src/mocks/__tests__/fixtures/export-golden.json`, so mock drift is caught
//! by the frontend CI job even when the Rust toolchain is unavailable.
//!
//! This file is where those goldens come from and stay honest:
//!
//! - `golden_outputs_match_rust_renderers` (runs in normal CI) fails if the
//!   committed goldens drift from the real renderers, forcing a refresh.
//! - `regenerate_goldens` (run explicitly) rewrites the goldens from the real
//!   Rust renderers after any renderer change:
//!
//!   cargo test --test export_goldens -- --ignored regenerate_goldens

use echo_note_lib::db::{Meeting, Summary, Transcript};
use echo_note_lib::export::{export_filename, render_meeting_markdown, render_meeting_text};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::path::PathBuf;

#[derive(Deserialize)]
struct FixtureCase {
    name: String,
    meeting: Meeting,
    transcript: Option<Transcript>,
    summary: Option<Summary>,
}

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<FixtureCase>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR must have a parent")
        .to_path_buf()
}

fn fixture_path() -> PathBuf {
    repo_root().join("src/mocks/__tests__/fixtures/export-meeting.json")
}

fn golden_path() -> PathBuf {
    repo_root().join("src/mocks/__tests__/fixtures/export-golden.json")
}

fn load_fixture() -> Fixture {
    let raw = std::fs::read_to_string(fixture_path())
        .expect("shared export fixture must exist (src/mocks/__tests__/fixtures/export-meeting.json)");
    serde_json::from_str(&raw).expect("shared export fixture must be valid JSON")
}

/// The exact shape `export-diff.mts` / the Vitest golden test expect per case.
fn case_golden(case: &FixtureCase) -> Value {
    json!({
        "markdown": render_meeting_markdown(&case.meeting, case.transcript.as_ref(), case.summary.as_ref()),
        "text": render_meeting_text(&case.meeting, case.transcript.as_ref(), case.summary.as_ref()),
        "filenameMd": export_filename(&case.meeting, "md"),
        "filenameTxt": export_filename(&case.meeting, "txt"),
    })
}

fn build_golden_map(fixture: &Fixture) -> Value {
    let mut map = Map::new();
    for case in &fixture.cases {
        map.insert(case.name.clone(), case_golden(case));
    }
    Value::Object(map)
}

/// CI guard: committed goldens must still equal what the real renderers
/// produce. Fails loudly with the regeneration command when they drift.
#[test]
fn golden_outputs_match_rust_renderers() {
    let fixture = load_fixture();
    let committed: Value = serde_json::from_str(
        &std::fs::read_to_string(golden_path())
            .expect("golden file must exist (src/mocks/__tests__/fixtures/export-golden.json) — run `cargo test --test export_goldens -- --ignored regenerate_goldens`"),
    )
    .expect("golden file must be valid JSON");
    let current = build_golden_map(&fixture);
    assert_eq!(
        committed, current,
        "committed export goldens drifted from the real Rust renderers — run `cargo test --test export_goldens -- --ignored regenerate_goldens` to refresh them"
    );
}

/// Explicitly rewrite the committed goldens from the real renderers. Ignored
/// by default so CI never silently regenerates expectations.
#[test]
#[ignore = "regenerate committed export goldens from the real Rust renderers after a renderer change"]
fn regenerate_goldens() {
    let fixture = load_fixture();
    let goldens = build_golden_map(&fixture);
    let pretty = serde_json::to_string_pretty(&goldens).expect("golden map must serialize");
    std::fs::write(golden_path(), format!("{pretty}\n")).expect("failed to write golden file");
}

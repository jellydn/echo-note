//! Differential fidelity test: the browser mock's export renderers
//! (`src/mocks/exportRenderers.ts`, exercised via `bun`) must produce
//! byte-identical output to the real Rust renderers for the same input.
//!
//! A shared JSON fixture (`src/mocks/__tests__/fixtures/export-meeting.json`)
//! is rendered through the real `render_meeting_markdown` /
//! `render_meeting_text`, then through the mock's `buildMarkdownExport` /
//! `buildTextExport`, and the two are asserted equal. This locks mock fidelity
//! into CI instead of relying on eyeballing the two implementations.

use echo_note_lib::db::{Meeting, Summary, Transcript};
use echo_note_lib::export::{export_filename, render_meeting_markdown, render_meeting_text};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

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

fn script_path() -> PathBuf {
    repo_root().join("scripts/export-diff.mts")
}

fn load_fixture() -> Fixture {
    let raw = std::fs::read_to_string(fixture_path())
        .expect("shared export fixture must exist (src/mocks/__tests__/fixtures/export-meeting.json)");
    serde_json::from_str(&raw).expect("shared export fixture must be valid JSON")
}

fn mock_outputs() -> Value {
    let output = Command::new("bun")
        .arg(script_path())
        .arg(fixture_path())
        .output()
        .expect("failed to run `bun scripts/export-diff.mts` — is bun installed and on PATH?");
    assert!(
        output.status.success(),
        "export-diff.mts failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("export-diff.mts must print valid JSON")
}

fn assert_case_matches(
    mock: &Value,
    case: &FixtureCase,
    rust_output: String,
    format: &str,
) {
    let mock_out = mock[&case.name][format]
        .as_str()
        .unwrap_or_else(|| panic!("mock output for case '{}' / {} must be a string", case.name, format));
    assert_eq!(
        rust_output, mock_out,
        "{} drift for case '{}'",
        format, case.name
    );
}

#[test]
fn mock_export_matches_rust_renderers_byte_for_byte() {
    let fixture = load_fixture();
    let mock = mock_outputs();

    for case in &fixture.cases {
        let rust_md = render_meeting_markdown(&case.meeting, case.transcript.as_ref(), case.summary.as_ref());
        let rust_text = render_meeting_text(&case.meeting, case.transcript.as_ref(), case.summary.as_ref());
        assert_case_matches(&mock, case, rust_md, "markdown");
        assert_case_matches(&mock, case, rust_text, "text");
        assert_case_matches(&mock, case, export_filename(&case.meeting, "md"), "filenameMd");
        assert_case_matches(&mock, case, export_filename(&case.meeting, "txt"), "filenameTxt");
    }
}

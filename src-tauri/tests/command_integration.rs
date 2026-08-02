//! Tauri command integration coverage (issue #36).
//!
//! Builds a command harness around [`AppStateExt`] with an isolated SQLite
//! pool and drives the real command handlers through a mock Tauri app. This
//! exercises the IPC contract — request deserialization, app-state wiring,
//! database access, and response shape — so regressions in command payloads
//! surface even when the Rust unit tests still pass.

mod common;

use common::setup_test_db;
use echo_note_lib::audio::AudioRecorder;
use echo_note_lib::commands::meetings::{
    create_meeting_command, delete_meeting_command, get_meeting_command, list_meetings_command,
    search_meetings_command, update_meeting_command, CreateMeetingRequest,
};
use echo_note_lib::commands::settings::{
    delete_setting_command, get_setting_command, list_settings_command, set_setting_command,
    GetSettingRequest, SetSettingRequest,
};
use echo_note_lib::commands::summaries::{
    create_summary_command, get_summary_by_meeting_command, CreateSummaryRequest,
};
use echo_note_lib::commands::transcripts::{
    create_transcript_command, get_transcript_by_meeting_command, CreateTranscriptRequest,
};
use echo_note_lib::keychain::{InMemorySecretStore, API_KEY_ACCOUNT};
use echo_note_lib::whisper::WhisperModelCache;
use echo_note_lib::AppStateExt;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// Build a mock Tauri app with real [`AppStateExt`] wired to an isolated pool.
async fn build_app() -> tauri::App<tauri::test::MockRuntime> {
    let pool = setup_test_db().await;
    let app = tauri::test::mock_app();
    app.manage(AppStateExt {
        db: pool,
        audio_recorder: Mutex::new(AudioRecorder::new()),
        secret_store: Arc::new(InMemorySecretStore::new()),
        whisper_model_cache: Arc::new(Mutex::new(WhisperModelCache::default())),
    });
    app
}

fn state<'a>(app: &'a tauri::App<tauri::test::MockRuntime>) -> tauri::State<'a, AppStateExt> {
    app.state::<AppStateExt>()
}

/// Settings round-trip: set, get, list, delete through the command layer.
#[tokio::test]
async fn settings_command_round_trip() {
    let app = build_app().await;

    let set = set_setting_command(
        state(&app),
        SetSettingRequest {
            key: "whisper_model_size".to_string(),
            value: "base".to_string(),
        },
    )
    .await
    .expect("set_setting_command should succeed");
    assert!(set.success && set.data == Some(true), "set should persist");

    let get = get_setting_command(
        state(&app),
        GetSettingRequest {
            key: "whisper_model_size".to_string(),
        },
    )
    .await
    .expect("get_setting_command should succeed");
    assert!(get.success, "get should succeed");
    assert_eq!(get.data.as_deref(), Some("base"));

    let list = list_settings_command(state(&app))
        .await
        .expect("list_settings_command should succeed");
    assert!(list.success);
    let keys: Vec<&str> = list
        .data
        .iter()
        .flatten()
        .map(|s| s.key.as_str())
        .collect();
    assert!(
        keys.contains(&"whisper_model_size"),
        "listed settings should include the one we set"
    );

    let del = delete_setting_command(state(&app), "whisper_model_size".to_string())
        .await
        .expect("delete_setting_command should succeed");
    assert!(del.success && del.data == Some(true));
}

/// Unknown settings keys fall back to their default rather than erroring.
#[tokio::test]
async fn get_setting_returns_default_for_missing_key() {
    let app = build_app().await;

    let get = get_setting_command(
        state(&app),
        GetSettingRequest {
            key: "audio_device".to_string(),
        },
    )
    .await
    .expect("get_setting_command should succeed");
    assert!(get.success);
    assert_eq!(get.data.as_deref(), Some("default"));
}

/// Secret keys (api_key) route to the secret store, never the database.
#[tokio::test]
async fn api_key_is_routed_to_secret_store_not_sqlite() {
    let app = build_app().await;

    let set = set_setting_command(
        state(&app),
        SetSettingRequest {
            key: API_KEY_ACCOUNT.to_string(),
            value: "sk-secret".to_string(),
        },
    )
    .await
    .expect("set_setting_command should succeed");
    assert!(set.success);

    let get = get_setting_command(
        state(&app),
        GetSettingRequest {
            key: API_KEY_ACCOUNT.to_string(),
        },
    )
    .await
    .expect("get_setting_command should succeed");
    assert_eq!(get.data.as_deref(), Some("sk-secret"));

    // The plaintext secret must never surface in the settings listing.
    let list = list_settings_command(state(&app))
        .await
        .expect("list_settings_command should succeed");
    let raw = serde_json::to_string(&list.data).unwrap_or_default();
    assert!(
        !raw.contains("sk-secret"),
        "secret value must not leak through the settings listing"
    );

    // And it must not be in the SQLite settings table either.
    let db_row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?1")
        .bind(API_KEY_ACCOUNT)
        .fetch_optional(&app.state::<AppStateExt>().db)
        .await
        .expect("query should succeed");
    assert!(db_row.is_none(), "api_key must not be persisted in SQLite");
}

/// Meeting CRUD through the command layer, including full-text search.
#[tokio::test]
async fn meeting_command_lifecycle_with_search() {
    let app = build_app().await;

    let created = create_meeting_command(
        state(&app),
        CreateMeetingRequest {
            title: "Q2 Planning".to_string(),
            date: chrono::Utc::now(),
            duration_seconds: 3600,
            audio_path: "/tmp/q2.wav".to_string(),
        },
    )
    .await
    .expect("create_meeting_command should succeed");
    assert!(created.success);
    let id = created.data.expect("created meeting").id;

    let fetched = get_meeting_command(state(&app), id)
        .await
        .expect("get_meeting_command should succeed");
    assert_eq!(fetched.data.unwrap().title, "Q2 Planning");

    let listed = list_meetings_command(state(&app))
        .await
        .expect("list_meetings_command should succeed");
    assert_eq!(listed.data.unwrap().len(), 1);

    // Add a transcript so the search index has content to match.
    let tr = create_transcript_command(
        state(&app),
        CreateTranscriptRequest {
            meeting_id: id,
            content: "discussed the quarterly budget".to_string(),
        },
    )
    .await
    .expect("create_transcript_command should succeed");
    assert!(tr.success);

    let hits = search_meetings_command(state(&app), "budget".to_string())
        .await
        .expect("search_meetings_command should succeed");
    assert_eq!(hits.data.unwrap().len(), 1, "search should find the meeting");

    let updated = update_meeting_command(state(&app), id, "Q2 Planning (renamed)".to_string())
        .await
        .expect("update_meeting_command should succeed");
    assert_eq!(updated.data.unwrap().title, "Q2 Planning (renamed)");

    let deleted = delete_meeting_command(state(&app), id)
        .await
        .expect("delete_meeting_command should succeed");
    assert!(deleted.success && deleted.data == Some(true));

    let after = list_meetings_command(state(&app))
        .await
        .expect("list_meetings_command should succeed");
    assert!(after.data.unwrap().is_empty(), "meeting should be deleted");
}

/// Transcript and summary commands wired to a meeting.
#[tokio::test]
async fn transcript_and_summary_commands_link_to_meeting() {
    let app = build_app().await;

    let meeting = create_meeting_command(
        state(&app),
        CreateMeetingRequest {
            title: "Sync".to_string(),
            date: chrono::Utc::now(),
            duration_seconds: 120,
            audio_path: "/tmp/sync.wav".to_string(),
        },
    )
    .await
    .expect("create_meeting_command should succeed");
    let meeting_id = meeting.data.unwrap().id;

    let transcript = create_transcript_command(
        state(&app),
        CreateTranscriptRequest {
            meeting_id,
            content: "Everyone agreed on next steps".to_string(),
        },
    )
    .await
    .expect("create_transcript_command should succeed");
    assert!(transcript.success);

    let by_meeting = get_transcript_by_meeting_command(state(&app), meeting_id)
        .await
        .expect("get_transcript_by_meeting_command should succeed");
    assert_eq!(
        by_meeting.data.as_ref().unwrap().as_ref().unwrap().content,
        "Everyone agreed on next steps"
    );

    let summary = create_summary_command(
        state(&app),
        CreateSummaryRequest {
            meeting_id,
            key_points: "- reviewed roadmap".to_string(),
            decisions: "ship by Friday".to_string(),
            action_items: "- Alice drafts notes".to_string(),
        },
    )
    .await
    .expect("create_summary_command should succeed");
    assert!(summary.success);

    let summary_by_meeting = get_summary_by_meeting_command(state(&app), meeting_id)
        .await
        .expect("get_summary_by_meeting_command should succeed");
    assert_eq!(
        summary_by_meeting
            .data
            .unwrap()
            .unwrap()
            .key_points,
        "- reviewed roadmap"
    );
}

/// Missing records are reported as typed API errors, not panics.
#[tokio::test]
async fn missing_records_return_error_responses() {
    let app = build_app().await;

    let missing = delete_setting_command(state(&app), "never_set".to_string())
        .await
        .expect("delete_setting_command should succeed");
    assert!(!missing.success, "deleting a missing setting should error");
    assert!(missing.error.is_some());
}

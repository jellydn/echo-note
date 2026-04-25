use echo_note_lib::db::*;

mod common;
use common::*;

#[tokio::test]
async fn test_create_meeting() {
    let pool = setup_test_db().await;

    let input = CreateMeetingInput {
        title: "Test Meeting".to_string(),
        date: chrono::Utc::now(),
        duration_seconds: 600,
        audio_path: "/test/recording.wav".to_string(),
    };

    let id = create_meeting(&pool, input)
        .await
        .expect("Failed to create meeting");
    assert!(id > 0);

    // Verify the meeting was created
    let meeting = get_meeting(&pool, id).await.expect("Failed to get meeting");
    assert!(meeting.is_some());
    let meeting = meeting.unwrap();
    assert_eq!(meeting.title, "Test Meeting");
    assert_eq!(meeting.duration_seconds, 600);
}

#[tokio::test]
async fn test_list_meetings() {
    let pool = setup_test_db().await;

    // Create multiple meetings
    let id1 = create_test_meeting(&pool, "Meeting 1").await;
    let id2 = create_test_meeting(&pool, "Meeting 2").await;
    let id3 = create_test_meeting(&pool, "Meeting 3").await;

    let meetings = list_meetings(&pool).await.expect("Failed to list meetings");
    assert_eq!(meetings.len(), 3);

    // Verify IDs are present
    let ids: Vec<i64> = meetings.iter().map(|m| m.id).collect();
    assert!(ids.contains(&id1));
    assert!(ids.contains(&id2));
    assert!(ids.contains(&id3));
}

#[tokio::test]
async fn test_update_meeting() {
    let pool = setup_test_db().await;

    let id = create_test_meeting(&pool, "Original Title").await;

    // Update the meeting
    let updated = update_meeting(&pool, id, "Updated Title".to_string())
        .await
        .expect("Failed to update meeting");
    assert!(updated);

    // Verify the update
    let meeting = get_meeting(&pool, id)
        .await
        .expect("Failed to get meeting")
        .unwrap();
    assert_eq!(meeting.title, "Updated Title");
}

#[tokio::test]
async fn test_delete_meeting() {
    let pool = setup_test_db().await;

    let id = create_test_meeting(&pool, "Meeting to Delete").await;

    // Verify meeting exists
    let meeting = get_meeting(&pool, id).await.expect("Failed to get meeting");
    assert!(meeting.is_some());

    // Delete the meeting
    let deleted = delete_meeting(&pool, id)
        .await
        .expect("Failed to delete meeting");
    assert!(deleted);

    // Verify meeting is gone
    let meeting = get_meeting(&pool, id).await.expect("Failed to get meeting");
    assert!(meeting.is_none());
}

#[tokio::test]
async fn test_create_transcript() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Test Meeting").await;

    let input = CreateTranscriptInput {
        meeting_id,
        content: "This is a test transcript.".to_string(),
    };

    let id = create_transcript(&pool, input)
        .await
        .expect("Failed to create transcript");
    assert!(id > 0);

    // Verify the transcript was created
    let transcript = get_transcript(&pool, id)
        .await
        .expect("Failed to get transcript");
    assert!(transcript.is_some());
    let transcript = transcript.unwrap();
    assert_eq!(transcript.content, "This is a test transcript.");
    assert_eq!(transcript.meeting_id, meeting_id);
}

#[tokio::test]
async fn test_get_transcript_by_meeting() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Test Meeting").await;
    let _transcript_id = create_test_transcript(&pool, meeting_id, "Test content").await;

    let transcript = get_transcript_by_meeting(&pool, meeting_id)
        .await
        .expect("Failed to get transcript by meeting")
        .expect("Transcript not found");

    assert_eq!(transcript.content, "Test content");
}

#[tokio::test]
async fn test_create_summary() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Test Meeting").await;

    let input = CreateSummaryInput {
        meeting_id,
        key_points: "- Point 1\n- Point 2".to_string(),
        decisions: "- Decision 1".to_string(),
        action_items: "- Action 1".to_string(),
    };

    let id = create_summary(&pool, input)
        .await
        .expect("Failed to create summary");
    assert!(id > 0);

    // Verify the summary was created
    let summary = get_summary(&pool, id).await.expect("Failed to get summary");
    assert!(summary.is_some());
    let summary = summary.unwrap();
    assert!(summary.key_points.contains("Point 1"));
    assert!(summary.decisions.contains("Decision 1"));
    assert!(summary.action_items.contains("Action 1"));
}

#[tokio::test]
async fn test_get_summary_by_meeting() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Test Meeting").await;
    let _summary_id = create_test_summary(
        &pool,
        meeting_id,
        "- Key point",
        "- Decision",
        "- Action item",
    )
    .await;

    let summary = get_summary_by_meeting(&pool, meeting_id)
        .await
        .expect("Failed to get summary by meeting")
        .expect("Summary not found");

    assert!(summary.key_points.contains("Key point"));
}

#[tokio::test]
async fn test_settings_crud() {
    let pool = setup_test_db().await;

    // Create a setting
    let created = set_setting(&pool, "test_key", "test_value")
        .await
        .expect("Failed to set setting");
    assert!(created);

    // Read the setting (with default value in case not found)
    let value = get_setting(&pool, "test_key", "default")
        .await
        .expect("Failed to get setting");
    assert_eq!(value, "test_value".to_string());

    // Update the setting
    let updated = set_setting(&pool, "test_key", "updated_value")
        .await
        .expect("Failed to update setting");
    assert!(updated);

    let value = get_setting(&pool, "test_key", "default")
        .await
        .expect("Failed to get setting");
    assert_eq!(value, "updated_value".to_string());

    // Delete the setting
    let deleted = delete_setting(&pool, "test_key")
        .await
        .expect("Failed to delete setting");
    assert!(deleted);

    // After deletion, should return default value
    let value = get_setting(&pool, "test_key", "default")
        .await
        .expect("Failed to get setting");
    assert_eq!(value, "default".to_string());
}

#[tokio::test]
async fn test_list_settings() {
    let pool = setup_test_db().await;

    // Create multiple settings
    set_setting(&pool, "key1", "value1").await.unwrap();
    set_setting(&pool, "key2", "value2").await.unwrap();
    set_setting(&pool, "key3", "value3").await.unwrap();

    let settings = list_settings(&pool).await.expect("Failed to list settings");
    assert_eq!(settings.len(), 3);
}

#[tokio::test]
async fn test_cascade_delete_meeting_deletes_transcript() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Test Meeting").await;
    let transcript_id = create_test_transcript(&pool, meeting_id, "Test content").await;

    // Verify transcript exists
    let transcript = get_transcript(&pool, transcript_id)
        .await
        .expect("Failed to get transcript");
    assert!(transcript.is_some());

    // Delete the meeting
    delete_meeting(&pool, meeting_id)
        .await
        .expect("Failed to delete meeting");

    // Verify transcript is also deleted (cascade)
    let transcript = get_transcript(&pool, transcript_id)
        .await
        .expect("Failed to get transcript");
    assert!(transcript.is_none());
}

#[tokio::test]
async fn test_update_nonexistent_meeting() {
    let pool = setup_test_db().await;

    // Try to update a meeting that doesn't exist
    let updated = update_meeting(&pool, 9999, "New Title".to_string())
        .await
        .expect("Failed to update meeting");
    assert!(!updated);
}

#[tokio::test]
async fn test_delete_nonexistent_meeting() {
    let pool = setup_test_db().await;

    // Try to delete a meeting that doesn't exist
    let deleted = delete_meeting(&pool, 9999)
        .await
        .expect("Failed to delete meeting");
    assert!(!deleted);
}

#[tokio::test]
async fn test_get_nonexistent_meeting() {
    let pool = setup_test_db().await;

    let meeting = get_meeting(&pool, 9999)
        .await
        .expect("Failed to get meeting");
    assert!(meeting.is_none());
}

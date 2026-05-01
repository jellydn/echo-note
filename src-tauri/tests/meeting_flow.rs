use echo_note_lib::db::*;

mod common;
use common::*;

/// Test the full meeting lifecycle: create → transcribe → summarize → delete
#[tokio::test]
async fn test_full_meeting_lifecycle() {
    let pool = setup_test_db().await;

    // 1. Create a meeting
    let meeting_input = CreateMeetingInput {
        title: "Quarterly Planning".to_string(),
        date: chrono::Utc::now(),
        duration_seconds: 3600,
        audio_path: "/recordings/quarterly_planning.wav".to_string(),
    };

    let meeting_id = create_meeting(&pool, meeting_input)
        .await
        .expect("Failed to create meeting");
    assert!(meeting_id > 0, "Meeting ID should be positive");

    // Verify meeting exists
    let meeting = get_meeting(&pool, meeting_id)
        .await
        .expect("Failed to get meeting")
        .expect("Meeting should exist");
    assert_eq!(meeting.title, "Quarterly Planning");

    // 2. Create a transcript for the meeting
    let transcript_input = CreateTranscriptInput {
        meeting_id,
        content: "We discussed Q3 goals and set targets for the team.".to_string(),
    };

    let transcript_id = create_transcript(&pool, transcript_input)
        .await
        .expect("Failed to create transcript");
    assert!(transcript_id > 0);

    // Verify transcript exists and is linked to meeting
    let transcript = get_transcript_by_meeting(&pool, meeting_id)
        .await
        .expect("Failed to get transcript")
        .expect("Transcript should exist");
    assert_eq!(
        transcript.content,
        "We discussed Q3 goals and set targets for the team."
    );

    // 3. Create a summary for the meeting
    let summary_input = CreateSummaryInput {
        meeting_id,
        key_points: "- Q3 revenue target: $1M\n- Launch mobile app".to_string(),
        decisions: "- Approved Q3 budget\n- Hired 2 developers".to_string(),
        action_items: "- John: Prepare roadmap\n- Sarah: Update team".to_string(),
    };

    let summary_id = create_summary(&pool, summary_input)
        .await
        .expect("Failed to create summary");
    assert!(summary_id > 0);

    // Verify summary exists and is linked to meeting
    let summary = get_summary_by_meeting(&pool, meeting_id)
        .await
        .expect("Failed to get summary")
        .expect("Summary should exist");
    assert!(summary.key_points.contains("$1M"));
    assert!(summary.decisions.contains("Approved Q3 budget"));

    // 4. Delete the meeting (should cascade to transcript and summary)
    let deleted = delete_meeting(&pool, meeting_id)
        .await
        .expect("Failed to delete meeting");
    assert!(deleted, "Meeting should be deleted");

    // Verify all related records are deleted
    let meeting = get_meeting(&pool, meeting_id)
        .await
        .expect("Failed to get meeting");
    assert!(meeting.is_none(), "Meeting should be deleted");

    let transcript = get_transcript(&pool, transcript_id)
        .await
        .expect("Failed to get transcript");
    assert!(
        transcript.is_none(),
        "Transcript should be deleted via cascade"
    );

    let summary = get_summary(&pool, summary_id)
        .await
        .expect("Failed to get summary");
    assert!(summary.is_none(), "Summary should be deleted via cascade");
}

/// Test meeting with empty transcript content (edge case)
#[tokio::test]
async fn test_meeting_with_empty_transcript() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Empty Transcript Meeting").await;

    // Create transcript with empty content
    let input = CreateTranscriptInput {
        meeting_id,
        content: "".to_string(),
    };

    let transcript_id = create_transcript(&pool, input)
        .await
        .expect("Should create empty transcript");

    let transcript = get_transcript(&pool, transcript_id).await.unwrap().unwrap();
    assert_eq!(transcript.content, "");
}

/// Test duplicate meeting titles are allowed
#[tokio::test]
async fn test_duplicate_meeting_titles() {
    let pool = setup_test_db().await;

    // Create two meetings with the same title
    let id1 = create_test_meeting(&pool, "Weekly Standup").await;
    let id2 = create_test_meeting(&pool, "Weekly Standup").await;

    // Both should be created with different IDs
    assert_ne!(id1, id2, "Meetings should have different IDs");

    let meetings = list_meetings(&pool).await.expect("Failed to list meetings");
    assert_eq!(meetings.len(), 2);

    // Both should exist with same title
    let meeting1 = get_meeting(&pool, id1).await.unwrap().unwrap();
    let meeting2 = get_meeting(&pool, id2).await.unwrap().unwrap();
    assert_eq!(meeting1.title, "Weekly Standup");
    assert_eq!(meeting2.title, "Weekly Standup");
}

/// Test updating meeting with transcript and summary intact
#[tokio::test]
async fn test_update_meeting_preserves_related_data() {
    let pool = setup_test_db().await;

    // Create full meeting data
    let meeting_id = create_test_meeting(&pool, "Original Title").await;
    let transcript_id = create_test_transcript(&pool, meeting_id, "Original content").await;
    let summary_id = create_test_summary(&pool, meeting_id, "Original key points", "", "").await;

    // Update only the meeting title
    let updated = update_meeting(&pool, meeting_id, "Updated Title".to_string())
        .await
        .expect("Failed to update meeting");
    assert!(updated);

    // Verify meeting title changed
    let meeting = get_meeting(&pool, meeting_id).await.unwrap().unwrap();
    assert_eq!(meeting.title, "Updated Title");

    // Verify transcript and summary are unchanged
    let transcript = get_transcript(&pool, transcript_id).await.unwrap().unwrap();
    assert_eq!(transcript.content, "Original content");

    let summary = get_summary(&pool, summary_id).await.unwrap().unwrap();
    assert!(summary.key_points.contains("Original key points"));
}

/// Test listing meetings returns them in correct order (newest first)
#[tokio::test]
async fn test_meetings_sorted_by_date() {
    let pool = setup_test_db().await;

    // Create meetings with specific dates
    let input1 = CreateMeetingInput {
        title: "Old Meeting".to_string(),
        date: chrono::DateTime::parse_from_rfc3339("2026-01-01T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        duration_seconds: 300,
        audio_path: "/test/old.wav".to_string(),
    };

    let input2 = CreateMeetingInput {
        title: "New Meeting".to_string(),
        date: chrono::DateTime::parse_from_rfc3339("2026-12-31T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        duration_seconds: 300,
        audio_path: "/test/new.wav".to_string(),
    };

    let id1 = create_meeting(&pool, input1).await.unwrap();
    let id2 = create_meeting(&pool, input2).await.unwrap();

    // List meetings - should be sorted by date (newest first)
    let meetings = list_meetings(&pool).await.unwrap();
    assert_eq!(meetings.len(), 2);
    assert_eq!(meetings[0].id, id2, "Newer meeting should be first");
    assert_eq!(meetings[1].id, id1, "Older meeting should be second");
}

/// Test multiple transcripts can exist for the same meeting
#[tokio::test]
async fn test_multiple_transcripts_per_meeting() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Test Meeting").await;

    // Create first transcript
    let input1 = CreateTranscriptInput {
        meeting_id,
        content: "First transcript".to_string(),
    };
    let id1 = create_transcript(&pool, input1).await.unwrap();

    // Create second transcript for same meeting
    let input2 = CreateTranscriptInput {
        meeting_id,
        content: "Second transcript".to_string(),
    };
    let id2 = create_transcript(&pool, input2).await.unwrap();

    // Verify both transcripts exist
    assert_ne!(id1, id2);
    let transcripts = list_transcripts(&pool).await.unwrap();
    assert_eq!(transcripts.len(), 2);
}

/// Test deleting transcript without affecting meeting
#[tokio::test]
async fn test_delete_transcript_preserves_meeting() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Test Meeting").await;
    let transcript_id = create_test_transcript(&pool, meeting_id, "Test content").await;

    // Delete only the transcript
    let deleted = delete_transcript(&pool, transcript_id)
        .await
        .expect("Failed to delete transcript");
    assert!(deleted);

    // Meeting should still exist
    let meeting = get_meeting(&pool, meeting_id).await.unwrap();
    assert!(
        meeting.is_some(),
        "Meeting should still exist after transcript deletion"
    );

    // Transcript should be gone
    let transcript = get_transcript(&pool, transcript_id).await.unwrap();
    assert!(transcript.is_none());
}

/// Test deleting summary without affecting meeting
#[tokio::test]
async fn test_delete_summary_preserves_meeting() {
    let pool = setup_test_db().await;

    let meeting_id = create_test_meeting(&pool, "Test Meeting").await;
    let summary_id =
        create_test_summary(&pool, meeting_id, "Key points", "Decisions", "Actions").await;

    // Delete only the summary
    let deleted = delete_summary(&pool, summary_id)
        .await
        .expect("Failed to delete summary");
    assert!(deleted);

    // Meeting should still exist
    let meeting = get_meeting(&pool, meeting_id).await.unwrap();
    assert!(
        meeting.is_some(),
        "Meeting should still exist after summary deletion"
    );

    // Summary should be gone
    let summary = get_summary(&pool, summary_id).await.unwrap();
    assert!(summary.is_none());
}

/// Test multiple meetings with full data
#[tokio::test]
async fn test_multiple_full_meetings() {
    let pool = setup_test_db().await;

    // Create 3 complete meetings (with transcripts and summaries)
    for i in 1..=3 {
        let meeting_id = create_test_meeting(&pool, &format!("Meeting {}", i)).await;
        create_test_transcript(&pool, meeting_id, &format!("Transcript {}", i)).await;
        create_test_summary(
            &pool,
            meeting_id,
            &format!("Key points {}", i),
            &format!("Decisions {}", i),
            &format!("Actions {}", i),
        )
        .await;
    }

    // Verify counts
    let meetings = list_meetings(&pool).await.unwrap();
    let transcripts = list_transcripts(&pool).await.unwrap();
    let summaries = list_summaries(&pool).await.unwrap();

    assert_eq!(meetings.len(), 3);
    assert_eq!(transcripts.len(), 3);
    assert_eq!(summaries.len(), 3);
}

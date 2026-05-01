use echo_note_lib::db::*;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

/// Create a temporary test database pool with shared in-memory database
/// Uses a unique UUID for each test to ensure complete isolation
/// while allowing multiple concurrent connections within that test
pub async fn setup_test_db() -> Pool<Sqlite> {
    // Generate a unique database name using UUID for guaranteed uniqueness
    let db_id = uuid::Uuid::new_v4();

    // Use shared cache in-memory database with unique name per test instance
    // This allows multiple connections to share the same database
    let url = format!("file:test_db_{}?mode=memory&cache=shared", db_id);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str(&url)
                .unwrap()
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .expect("Failed to create test database pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Helper to create a test meeting
pub async fn create_test_meeting(pool: &Pool<Sqlite>, title: &str) -> i64 {
    let input = CreateMeetingInput {
        title: title.to_string(),
        date: chrono::Utc::now(),
        duration_seconds: 300,
        audio_path: "/test/audio.wav".to_string(),
    };

    create_meeting(pool, input)
        .await
        .expect("Failed to create test meeting")
}

/// Helper to create a test transcript
pub async fn create_test_transcript(pool: &Pool<Sqlite>, meeting_id: i64, content: &str) -> i64 {
    let input = CreateTranscriptInput {
        meeting_id,
        content: content.to_string(),
    };

    create_transcript(pool, input)
        .await
        .expect("Failed to create test transcript")
}

/// Helper to create a test summary
pub async fn create_test_summary(
    pool: &Pool<Sqlite>,
    meeting_id: i64,
    key_points: &str,
    decisions: &str,
    action_items: &str,
) -> i64 {
    let input = CreateSummaryInput {
        meeting_id,
        key_points: key_points.to_string(),
        decisions: decisions.to_string(),
        action_items: action_items.to_string(),
    };

    create_summary(pool, input)
        .await
        .expect("Failed to create test summary")
}

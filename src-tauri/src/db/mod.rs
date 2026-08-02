use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::str::FromStr;
use tauri::Manager;

/// Meeting record from the database
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Meeting {
    pub id: i64,
    pub title: String,
    pub date: DateTime<Utc>,
    pub duration_seconds: i64,
    pub audio_path: String,
    pub created_at: DateTime<Utc>,
}

/// Data needed to create a new meeting
#[derive(Debug, Deserialize)]
pub struct CreateMeetingInput {
    pub title: String,
    pub date: DateTime<Utc>,
    pub duration_seconds: i64,
    pub audio_path: String,
}

/// Transcript record from the database
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Transcript {
    pub id: i64,
    pub meeting_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Data needed to create a new transcript
#[derive(Debug, Deserialize)]
pub struct CreateTranscriptInput {
    pub meeting_id: i64,
    pub content: String,
}

/// Summary record from the database
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Summary {
    pub id: i64,
    pub meeting_id: i64,
    pub key_points: String,
    pub decisions: String,
    pub action_items: String,
    pub created_at: DateTime<Utc>,
}

/// Data needed to create a new summary
#[derive(Debug, Deserialize)]
pub struct CreateSummaryInput {
    pub meeting_id: i64,
    pub key_points: String,
    pub decisions: String,
    pub action_items: String,
}

/// Setting record from the database
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Setting {
    pub id: i64,
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Default settings keys
pub const DEFAULT_AUDIO_DEVICE: &str = "default";
pub const DEFAULT_WHISPER_MODEL_SIZE: &str = "small";
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";
pub const DEFAULT_API_KEY: &str = "";
pub const DEFAULT_API_ENDPOINT: &str = "https://api.openai.com/v1";
pub const DEFAULT_FIRST_LAUNCH_COMPLETED: &str = "false";
pub const DEFAULT_BLACKHOLE_INSTALL_ATTEMPTED: &str = "false";
pub const DEFAULT_API_MODEL: &str = "gpt-4o-mini";
pub const DEFAULT_DIARIZATION_ENABLED: &str = "true";
pub const DEFAULT_DIARIZATION_THRESHOLD: &str = "0.75";
pub const DEFAULT_AUDIO_RETENTION_DAYS: &str = "30";

/// Initialize the database pool and create tables if they don't exist
pub async fn init_db(app_handle: &tauri::AppHandle) -> Result<Pool<Sqlite>> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .context("Failed to get app data directory")?;

    // Ensure the app data directory exists
    std::fs::create_dir_all(&app_dir).context("Failed to create app data directory")?;

    let db_path = app_dir.join("echo_note.db");
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());
    let connect_options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    // Run migrations from src-tauri/migrations/
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;

    Ok(pool)
}

/// Create a new meeting record
pub async fn create_meeting(pool: &Pool<Sqlite>, input: CreateMeetingInput) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO meetings (title, date, duration_seconds, audio_path)
        VALUES (?1, ?2, ?3, ?4)
        "#,
    )
    .bind(&input.title)
    .bind(input.date)
    .bind(input.duration_seconds)
    .bind(&input.audio_path)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get a single meeting by ID
pub async fn get_meeting(pool: &Pool<Sqlite>, id: i64) -> Result<Option<Meeting>> {
    let meeting = sqlx::query_as::<_, Meeting>(
        r#"
        SELECT id, title, date, duration_seconds, audio_path, created_at
        FROM meetings
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(meeting)
}

/// List all meetings, sorted by date (newest first)
pub async fn list_meetings(pool: &Pool<Sqlite>) -> Result<Vec<Meeting>> {
    let meetings = sqlx::query_as::<_, Meeting>(
        r#"
        SELECT id, title, date, duration_seconds, audio_path, created_at
        FROM meetings
        ORDER BY date DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(meetings)
}

/// Full-text search over meeting titles, transcripts, and summaries using FTS5.
/// Results are ranked by relevance and returned newest-first as a tiebreaker.
/// Build a safe FTS5 phrase query from raw user input.
///
/// Doubles embedded double quotes (FTS5's escape for a literal quote inside a
/// quoted phrase) and wraps the whole term in quotes so the text is matched
/// literally instead of being parsed as FTS5 operators (AND/OR/NOT), which
/// could otherwise abort the search with a syntax error.
pub(crate) fn fts_phrase_query(query: &str) -> String {
    let escaped = query.trim().replace('"', "\"\"");
    format!("\"{escaped}\"")
}

pub async fn search_meetings(pool: &Pool<Sqlite>, query: &str) -> Result<Vec<Meeting>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return list_meetings(pool).await;
    }

    // Escape the query so user input is treated as literal text, then wrap it
    // in double quotes so FTS5 performs a phrase match.
    let fts_query = fts_phrase_query(trimmed);

    let meetings = sqlx::query_as::<_, Meeting>(
        r#"
        SELECT m.id, m.title, m.date, m.duration_seconds, m.audio_path, m.created_at
        FROM meeting_search
        JOIN meetings m ON m.id = meeting_search.rowid
        WHERE meeting_search MATCH ?1
        ORDER BY bm25(meeting_search), m.date DESC
        "#,
    )
    .bind(fts_query)
    .fetch_all(pool)
    .await?;

    Ok(meetings)
}

/// Update a meeting's title
pub async fn update_meeting(pool: &Pool<Sqlite>, id: i64, title: String) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE meetings
        SET title = ?1
        WHERE id = ?2
        "#,
    )
    .bind(&title)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete a meeting by ID
pub async fn delete_meeting(pool: &Pool<Sqlite>, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM meetings WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// ==================== TRANSCRIPT CRUD ====================

/// Create a new transcript record
pub async fn create_transcript(pool: &Pool<Sqlite>, input: CreateTranscriptInput) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO transcripts (meeting_id, content)
        VALUES (?1, ?2)
        "#,
    )
    .bind(input.meeting_id)
    .bind(&input.content)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get a transcript by ID
pub async fn get_transcript(pool: &Pool<Sqlite>, id: i64) -> Result<Option<Transcript>> {
    let transcript = sqlx::query_as::<_, Transcript>(
        r#"
        SELECT id, meeting_id, content, created_at
        FROM transcripts
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(transcript)
}

/// Get transcript by meeting ID (one-to-one relationship)
pub async fn get_transcript_by_meeting(
    pool: &Pool<Sqlite>,
    meeting_id: i64,
) -> Result<Option<Transcript>> {
    let transcript = sqlx::query_as::<_, Transcript>(
        r#"
        SELECT id, meeting_id, content, created_at
        FROM transcripts
        WHERE meeting_id = ?1
        "#,
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await?;

    Ok(transcript)
}

/// List all transcripts
pub async fn list_transcripts(pool: &Pool<Sqlite>) -> Result<Vec<Transcript>> {
    let transcripts = sqlx::query_as::<_, Transcript>(
        r#"
        SELECT id, meeting_id, content, created_at
        FROM transcripts
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(transcripts)
}

/// Update a transcript's content
pub async fn update_transcript(pool: &Pool<Sqlite>, id: i64, content: String) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE transcripts
        SET content = ?1
        WHERE id = ?2
        "#,
    )
    .bind(&content)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete a transcript by ID
pub async fn delete_transcript(pool: &Pool<Sqlite>, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM transcripts WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// ==================== SUMMARY CRUD ====================

/// Create a new summary record
pub async fn create_summary(pool: &Pool<Sqlite>, input: CreateSummaryInput) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO summaries (meeting_id, key_points, decisions, action_items)
        VALUES (?1, ?2, ?3, ?4)
        "#,
    )
    .bind(input.meeting_id)
    .bind(&input.key_points)
    .bind(&input.decisions)
    .bind(&input.action_items)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get a summary by ID
pub async fn get_summary(pool: &Pool<Sqlite>, id: i64) -> Result<Option<Summary>> {
    let summary = sqlx::query_as::<_, Summary>(
        r#"
        SELECT id, meeting_id, key_points, decisions, action_items, created_at
        FROM summaries
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(summary)
}

/// Get summary by meeting ID (one-to-one relationship)
pub async fn get_summary_by_meeting(
    pool: &Pool<Sqlite>,
    meeting_id: i64,
) -> Result<Option<Summary>> {
    let summary = sqlx::query_as::<_, Summary>(
        r#"
        SELECT id, meeting_id, key_points, decisions, action_items, created_at
        FROM summaries
        WHERE meeting_id = ?1
        "#,
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await?;

    Ok(summary)
}

/// List all summaries
pub async fn list_summaries(pool: &Pool<Sqlite>) -> Result<Vec<Summary>> {
    let summaries = sqlx::query_as::<_, Summary>(
        r#"
        SELECT id, meeting_id, key_points, decisions, action_items, created_at
        FROM summaries
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(summaries)
}

/// Update a summary
pub async fn update_summary(
    pool: &Pool<Sqlite>,
    id: i64,
    key_points: String,
    decisions: String,
    action_items: String,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE summaries
        SET key_points = ?1, decisions = ?2, action_items = ?3
        WHERE id = ?4
        "#,
    )
    .bind(&key_points)
    .bind(&decisions)
    .bind(&action_items)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete a summary by ID
pub async fn delete_summary(pool: &Pool<Sqlite>, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM summaries WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// ==================== SETTINGS CRUD ====================

/// Get a setting by key, returning the default value if not found
pub async fn get_setting(pool: &Pool<Sqlite>, key: &str, default_value: &str) -> Result<String> {
    let setting: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT value FROM settings WHERE key = ?1
        "#,
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    Ok(setting
        .map(|s| s.0)
        .unwrap_or_else(|| default_value.to_string()))
}

/// Set a setting value (insert or update)
pub async fn set_setting(pool: &Pool<Sqlite>, key: &str, value: &str) -> Result<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO settings (key, value, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(key)
    .bind(value)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Get all settings as a vector of Setting structs
pub async fn list_settings(pool: &Pool<Sqlite>) -> Result<Vec<Setting>> {
    let settings = sqlx::query_as::<_, Setting>(
        r#"
        SELECT id, key, value, created_at, updated_at
        FROM settings
        ORDER BY key ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(settings)
}

/// Delete a setting by key
pub async fn delete_setting(pool: &Pool<Sqlite>, key: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM settings WHERE key = ?1")
        .bind(key)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Initialize default settings if they don't exist
pub async fn init_default_settings(pool: &Pool<Sqlite>) -> Result<()> {
    // NOTE: api_key is intentionally absent — secrets live in the macOS
    // Keychain (see `keychain` module), never in the SQLite settings table.
    let defaults = [
        ("audio_device", DEFAULT_AUDIO_DEVICE),
        ("whisper_model_size", DEFAULT_WHISPER_MODEL_SIZE),
        ("llm_provider", DEFAULT_LLM_PROVIDER),
        ("api_endpoint", DEFAULT_API_ENDPOINT),
        ("first_launch_completed", DEFAULT_FIRST_LAUNCH_COMPLETED),
        (
            "blackhole_install_attempted",
            DEFAULT_BLACKHOLE_INSTALL_ATTEMPTED,
        ),
        ("api_model", DEFAULT_API_MODEL),
        ("diarization_enabled", DEFAULT_DIARIZATION_ENABLED),
        ("diarization_threshold", DEFAULT_DIARIZATION_THRESHOLD),
        ("audio_retention_days", DEFAULT_AUDIO_RETENTION_DAYS),
    ];

    for (key, default_value) in &defaults {
        // Only insert if the key doesn't exist
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO settings (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            "#,
        )
        .bind(key)
        .bind(*default_value)
        .bind(Utc::now())
        .execute(pool)
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_phrase_query_wraps_plain_text() {
        assert_eq!(fts_phrase_query("weekly sync"), "\"weekly sync\"");
    }

    #[test]
    fn fts_phrase_query_trims_whitespace() {
        assert_eq!(fts_phrase_query("  q2 plan  "), "\"q2 plan\"");
    }

    #[test]
    fn fts_phrase_query_escapes_embedded_quotes() {
        // FTS5 doubles a quote inside a quoted phrase to match it literally
        assert_eq!(fts_phrase_query("say \"hi\" now"), "\"say \"\"hi\"\" now\"");
    }

    #[test]
    fn fts_phrase_query_handles_operators_as_literal() {
        // FTS5 operators are escaped so a bare user query is never parsed
        assert_eq!(fts_phrase_query("budget AND costs"), "\"budget AND costs\"");
    }
}

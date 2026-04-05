use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
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

/// Application state containing the database pool
pub struct AppState {
    pub db: Pool<Sqlite>,
}

/// Initialize the database pool and create tables if they don't exist
pub async fn init_db(app_handle: &tauri::AppHandle) -> Result<Pool<Sqlite>> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");

    // Ensure the app data directory exists
    std::fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("echo_note.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

    Ok(pool)
}

/// Run database migrations
async fn run_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS meetings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            date DATETIME NOT NULL,
            duration_seconds INTEGER NOT NULL,
            audio_path TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
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

/// Delete a meeting by ID
pub async fn delete_meeting(pool: &Pool<Sqlite>, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM meetings WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

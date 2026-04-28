use std::str::FromStr;

use chrono::Utc;
use sqlx::{
    FromRow, Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Clone, Debug, FromRow, serde::Serialize)]
pub struct ConversationRecord {
    pub id: String,
    pub title: String,
    pub title_generated: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, FromRow, serde::Serialize)]
pub struct MessageRecord {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, AppError> {
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(AppError::database)?
        .create_if_missing(true)
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(AppError::database)
}

pub async fn init_db(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await
        .map_err(AppError::database)?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            title_generated INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::database)?;

    ensure_title_generated_column(pool).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::database)?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_messages_conversation_created ON messages(conversation_id, created_at)",
    )
    .execute(pool)
    .await
    .map_err(AppError::database)?;

    Ok(())
}

async fn ensure_title_generated_column(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(conversations)")
        .fetch_all(pool)
        .await
        .map_err(AppError::database)?;
    let has_column = columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "title_generated");

    if !has_column {
        sqlx::query(
            "ALTER TABLE conversations ADD COLUMN title_generated INTEGER NOT NULL DEFAULT 0",
        )
        .execute(pool)
        .await
        .map_err(AppError::database)?;
    }

    Ok(())
}

pub async fn list_conversations(pool: &SqlitePool) -> Result<Vec<ConversationRecord>, AppError> {
    sqlx::query_as::<_, ConversationRecord>(
        "SELECT id, title, title_generated, created_at, updated_at FROM conversations ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::database)
}

pub async fn create_conversation(
    pool: &SqlitePool,
    title: &str,
    title_generated: bool,
) -> Result<ConversationRecord, AppError> {
    let now = now();
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO conversations (id, title, title_generated, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&id)
    .bind(title)
    .bind(title_generated)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(AppError::database)?;

    get_conversation(pool, &id)
        .await?
        .ok_or_else(|| AppError::not_found("conversation not found after create"))
}

pub async fn get_conversation(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<ConversationRecord>, AppError> {
    sqlx::query_as::<_, ConversationRecord>(
        "SELECT id, title, title_generated, created_at, updated_at FROM conversations WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::database)
}

pub async fn list_messages(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<MessageRecord>, AppError> {
    sqlx::query_as::<_, MessageRecord>(
        r#"
        SELECT id, conversation_id, role, content, created_at
        FROM messages
        WHERE conversation_id = ?1
        ORDER BY created_at ASC
        "#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::database)
}

pub async fn create_message(
    pool: &SqlitePool,
    conversation_id: &str,
    role: &str,
    content: &str,
) -> Result<MessageRecord, AppError> {
    let now = now();
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO messages (id, conversation_id, role, content, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
    )
    .bind(&id)
    .bind(conversation_id)
    .bind(role)
    .bind(content)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(AppError::database)?;

    touch_conversation(pool, conversation_id).await?;

    sqlx::query_as::<_, MessageRecord>(
        "SELECT id, conversation_id, role, content, created_at FROM messages WHERE id = ?1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::database)
}

pub async fn maybe_set_conversation_title(
    pool: &SqlitePool,
    conversation: &ConversationRecord,
    content: &str,
) -> Result<ConversationRecord, AppError> {
    if !conversation.title_generated {
        return Ok(conversation.clone());
    }

    let title = content
        .lines()
        .next()
        .unwrap_or("New chat")
        .trim()
        .chars()
        .take(60)
        .collect::<String>();
    let title = if title.is_empty() { "New chat" } else { &title };

    sqlx::query(
        "UPDATE conversations SET title = ?1, title_generated = 0, updated_at = ?2 WHERE id = ?3",
    )
    .bind(title)
    .bind(now())
    .bind(&conversation.id)
    .execute(pool)
    .await
    .map_err(AppError::database)?;

    get_conversation(pool, &conversation.id)
        .await?
        .ok_or_else(|| AppError::not_found("conversation not found after title update"))
}

async fn touch_conversation(pool: &SqlitePool, conversation_id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE conversations SET updated_at = ?1 WHERE id = ?2")
        .bind(now())
        .bind(conversation_id)
        .execute(pool)
        .await
        .map_err(AppError::database)?;
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

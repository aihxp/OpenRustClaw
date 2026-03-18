//! Durable session and conversation persistence helpers.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use openrustclaw_core::error::{DatabaseError, Error, Result};
use openrustclaw_core::types::{Message, Platform, Role, Session, SessionType, ToolCall};

use crate::models::{ConversationRow, SessionRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Archived,
    Closed,
}

impl SessionStatus {
    fn as_str(self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Archived => "archived",
            SessionStatus::Closed => "closed",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "archived" => SessionStatus::Archived,
            "closed" => SessionStatus::Closed,
            _ => SessionStatus::Active,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedSession {
    pub session: Session,
    pub status: SessionStatus,
    pub route_key: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct SqliteSessionStore {
    pool: SqlitePool,
}

impl std::fmt::Debug for SqliteSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteSessionStore")
            .field("pool", &"<SqlitePool>")
            .finish()
    }
}

impl SqliteSessionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_or_update(
        &self,
        session: &Session,
        route_key: Option<&str>,
        status: SessionStatus,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (
                id, session_type, user_id, channel, workspace_id, created_at, updated_at,
                metadata, status, route_key, archived_at, closed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL)
            ON CONFLICT(id) DO UPDATE SET
                session_type = excluded.session_type,
                user_id = excluded.user_id,
                channel = excluded.channel,
                workspace_id = excluded.workspace_id,
                updated_at = excluded.updated_at,
                metadata = excluded.metadata,
                status = excluded.status,
                route_key = COALESCE(excluded.route_key, sessions.route_key)
            "#,
        )
        .bind(session.id.to_string())
        .bind(session_type_to_string(session.session_type))
        .bind(&session.user_id)
        .bind(platform_to_string(session.channel))
        .bind(session.workspace_id.as_ref())
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .bind(serialize_json(&session.metadata)?)
        .bind(status.as_str())
        .bind(route_key)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to persist session: {}",
                e
            )))
        })?;

        Ok(())
    }

    pub async fn touch(&self, session_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to touch session: {}",
                e
            )))
        })?;
        Ok(())
    }

    pub async fn find_active_by_route_key(
        &self,
        route_key: &str,
    ) -> Result<Option<PersistedSession>> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT * FROM sessions
            WHERE route_key = ?
              AND status = 'active'
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(route_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to load session by route key: {}",
                e
            )))
        })?;

        match row {
            Some(row) => self.row_to_persisted_session(row).map(Some),
            None => Ok(None),
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<PersistedSession>> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT * FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to load session: {}",
                e
            )))
        })?;

        match row {
            Some(row) => self.row_to_persisted_session(row).map(Some),
            None => Ok(None),
        }
    }

    pub async fn list_sessions(
        &self,
        status: Option<SessionStatus>,
        limit: usize,
    ) -> Result<Vec<PersistedSession>> {
        let rows = match status {
            Some(status) => {
                sqlx::query_as::<_, SessionRow>(
                    r#"
                    SELECT * FROM sessions
                    WHERE status = ?
                    ORDER BY updated_at DESC
                    LIMIT ?
                    "#,
                )
                .bind(status.as_str())
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, SessionRow>(
                    r#"
                    SELECT * FROM sessions
                    ORDER BY updated_at DESC
                    LIMIT ?
                    "#,
                )
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list sessions: {}",
                e
            )))
        })?;

        rows.into_iter()
            .map(|row| self.row_to_persisted_session(row))
            .collect()
    }

    pub async fn append_message(&self, session_id: &str, message: &Message) -> Result<()> {
        let tool_calls = message
            .tool_calls
            .as_ref()
            .map(serialize_json)
            .transpose()?;
        sqlx::query(
            r#"
            INSERT INTO conversations (
                id, session_id, role, content, tool_calls, tool_call_id, token_count, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO NOTHING
            "#,
        )
        .bind(message.id.to_string())
        .bind(session_id)
        .bind(role_to_string(message.role))
        .bind(&message.content)
        .bind(tool_calls)
        .bind(message.tool_call_id.as_ref())
        .bind(message.token_count.map(|value| value as i64))
        .bind(message.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to append conversation message: {}",
                e
            )))
        })?;

        self.touch(session_id).await
    }

    pub async fn list_history(&self, session_id: &str, limit: usize) -> Result<Vec<Message>> {
        let rows = sqlx::query_as::<_, ConversationRow>(
            r#"
            SELECT * FROM conversations
            WHERE session_id = ?
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to list session history: {}",
                e
            )))
        })?;

        rows.into_iter().map(row_to_message).collect()
    }

    pub async fn set_status(
        &self,
        session_id: &str,
        status: SessionStatus,
        reason: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let archived_at = (status == SessionStatus::Archived).then_some(now.clone());
        let closed_at = (status == SessionStatus::Closed).then_some(now.clone());
        let metadata = if let Some(existing) = self.get_session(session_id).await? {
            let mut metadata = existing.session.metadata;
            if let Some(reason) = reason {
                metadata["status_reason"] = serde_json::Value::String(reason.to_string());
            }
            serialize_json(&metadata)?
        } else {
            serde_json::json!({
                "status_reason": reason.unwrap_or_default()
            })
            .to_string()
        };

        sqlx::query(
            r#"
            UPDATE sessions
            SET status = ?,
                updated_at = ?,
                archived_at = COALESCE(?, archived_at),
                closed_at = COALESCE(?, closed_at),
                metadata = ?
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(&now)
        .bind(archived_at)
        .bind(closed_at)
        .bind(metadata)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::Database(DatabaseError::Query(format!(
                "Failed to update session status: {}",
                e
            )))
        })?;

        Ok(())
    }

    fn row_to_persisted_session(&self, row: SessionRow) -> Result<PersistedSession> {
        let metadata = parse_json_opt(row.metadata.as_deref())?;
        Ok(PersistedSession {
            session: Session {
                id: Uuid::parse_str(&row.id).map_err(|e| {
                    Error::Database(DatabaseError::Query(format!("Invalid session id: {}", e)))
                })?,
                session_type: parse_session_type(&row.session_type),
                user_id: row.user_id,
                channel: parse_platform(&row.channel),
                workspace_id: row.workspace_id,
                created_at: parse_time(&row.created_at)?,
                updated_at: parse_time(&row.updated_at)?,
                metadata,
            },
            status: SessionStatus::parse(row.status.as_deref().unwrap_or("active")),
            route_key: row.route_key,
            archived_at: row.archived_at.as_deref().map(parse_time).transpose()?,
            closed_at: row.closed_at.as_deref().map(parse_time).transpose()?,
        })
    }
}

fn serialize_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|e| {
        Error::Database(DatabaseError::Query(format!(
            "Failed to serialize JSON: {}",
            e
        )))
    })
}

fn parse_json_opt(value: Option<&str>) -> Result<serde_json::Value> {
    match value {
        Some(value) if !value.trim().is_empty() => serde_json::from_str(value).map_err(|e| {
            Error::Database(DatabaseError::Query(format!("Failed to parse JSON: {}", e)))
        }),
        _ => Ok(serde_json::json!({})),
    }
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|e| Error::Database(DatabaseError::Query(format!("Invalid timestamp: {}", e))))
}

fn row_to_message(row: ConversationRow) -> Result<Message> {
    Ok(Message {
        id: Uuid::parse_str(&row.id).map_err(|e| {
            Error::Database(DatabaseError::Query(format!("Invalid message id: {}", e)))
        })?,
        role: parse_role(&row.role),
        content: row.content,
        tool_calls: row
            .tool_calls
            .as_deref()
            .map(serde_json::from_str::<Vec<ToolCall>>)
            .transpose()
            .map_err(|e| {
                Error::Database(DatabaseError::Query(format!(
                    "Invalid tool_calls JSON: {}",
                    e
                )))
            })?,
        tool_call_id: row.tool_call_id,
        token_count: row.token_count.map(|value| value as usize),
        created_at: parse_time(&row.created_at)?,
    })
}

fn session_type_to_string(value: SessionType) -> &'static str {
    match value {
        SessionType::Dm => "dm",
        SessionType::Group => "group",
        SessionType::Isolated => "isolated",
    }
}

fn parse_session_type(value: &str) -> SessionType {
    match value {
        "group" => SessionType::Group,
        "isolated" => SessionType::Isolated,
        _ => SessionType::Dm,
    }
}

fn role_to_string(value: Role) -> &'static str {
    match value {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

fn parse_role(value: &str) -> Role {
    match value {
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "tool" => Role::Tool,
        _ => Role::User,
    }
}

fn platform_to_string(value: Platform) -> &'static str {
    match value {
        Platform::WebChat => "web_chat",
        Platform::Telegram => "telegram",
        Platform::Discord => "discord",
        Platform::Slack => "slack",
        Platform::WhatsApp => "whatsapp",
        Platform::Teams => "teams",
        Platform::GoogleChat => "google_chat",
        Platform::Gmail => "gmail",
        Platform::Twilio => "twilio",
        Platform::Signal => "signal",
        Platform::Matrix => "matrix",
        Platform::X => "x",
        Platform::Messenger => "messenger",
        Platform::Instagram => "instagram",
        Platform::IMessage => "imessage",
        Platform::Line => "line",
        Platform::Viber => "viber",
        Platform::WeChat => "wechat",
        Platform::Cli => "cli",
        Platform::Api => "api",
    }
}

fn parse_platform(value: &str) -> Platform {
    match value {
        "telegram" => Platform::Telegram,
        "discord" => Platform::Discord,
        "slack" => Platform::Slack,
        "whatsapp" => Platform::WhatsApp,
        "teams" => Platform::Teams,
        "google_chat" => Platform::GoogleChat,
        "gmail" => Platform::Gmail,
        "twilio" => Platform::Twilio,
        "signal" => Platform::Signal,
        "matrix" => Platform::Matrix,
        "x" => Platform::X,
        "messenger" => Platform::Messenger,
        "instagram" => Platform::Instagram,
        "imessage" => Platform::IMessage,
        "line" => Platform::Line,
        "viber" => Platform::Viber,
        "wechat" => Platform::WeChat,
        "cli" => Platform::Cli,
        "api" => Platform::Api,
        _ => Platform::WebChat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::types::Message;

    #[tokio::test]
    async fn session_store_roundtrip() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::run_migrations(&pool).await.unwrap();
        let store = SqliteSessionStore::new(pool);

        let mut session = Session::new(SessionType::Dm, "user-1", Platform::Cli);
        session.metadata = serde_json::json!({"route_key": "cli:main:user-1"});
        store
            .create_or_update(&session, Some("cli:main:user-1"), SessionStatus::Active)
            .await
            .unwrap();
        store
            .append_message(&session.id.to_string(), &Message::user("hello"))
            .await
            .unwrap();
        store
            .append_message(&session.id.to_string(), &Message::assistant("hi"))
            .await
            .unwrap();

        let loaded = store
            .get_session(&session.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.session.user_id, "user-1");

        let history = store
            .list_history(&session.id.to_string(), 10)
            .await
            .unwrap();
        assert_eq!(history.len(), 2);

        let sessions = store
            .list_sessions(Some(SessionStatus::Active), 10)
            .await
            .unwrap();
        assert_eq!(sessions.len(), 1);
    }
}

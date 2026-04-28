use std::{convert::Infallible, sync::Arc};

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bytes::Bytes;
use futures_util::StreamExt;
use serde_json::json;
use sqlx::SqlitePool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    db,
    error::AppError,
    models::{
        ConversationDto, ConversationListResponse, CreateConversationRequest,
        CreateConversationResponse, MessageDto, MessageListResponse, SendMessageRequest,
        SendMessageResponse, StreamEvent,
    },
    openrouter::{ChatProvider, ProviderMessage},
};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub chat_provider: Arc<dyn ChatProvider>,
}

impl AppState {
    pub fn new(pool: SqlitePool, chat_provider: Arc<dyn ChatProvider>) -> Self {
        Self {
            pool,
            chat_provider,
        }
    }
}

struct PreparedProviderTurn {
    conversation: db::ConversationRecord,
    provider_messages: Vec<ProviderMessage>,
}

pub fn create_app(state: AppState, frontend_origin: &str) -> Result<Router, AppError> {
    let origin = frontend_origin
        .parse::<HeaderValue>()
        .map_err(|_| AppError::config("FRONTEND_ORIGIN must be a valid HTTP origin"))?;

    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE]);

    Ok(Router::new()
        .route("/health", get(health))
        .route(
            "/api/conversations",
            get(list_conversations).post(create_conversation),
        )
        .route(
            "/api/conversations/{conversation_id}/messages",
            get(list_messages).post(send_message),
        )
        .route(
            "/api/conversations/{conversation_id}/messages/stream",
            post(stream_message),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn list_conversations(
    State(state): State<AppState>,
) -> Result<Json<ConversationListResponse>, AppError> {
    let conversations = db::list_conversations(&state.pool)
        .await?
        .into_iter()
        .map(ConversationDto::from)
        .collect();
    Ok(Json(ConversationListResponse { conversations }))
}

async fn create_conversation(
    State(state): State<AppState>,
    payload: Result<Json<CreateConversationRequest>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let Json(payload) = payload.map_err(AppError::invalid_json)?;
    let (title, title_generated) = validate_title(payload.title)?;
    let conversation = db::create_conversation(&state.pool, &title, title_generated).await?;
    Ok((
        StatusCode::CREATED,
        Json(CreateConversationResponse {
            conversation: conversation.into(),
        }),
    ))
}

async fn list_messages(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<MessageListResponse>, AppError> {
    ensure_conversation(&state, &conversation_id).await?;
    let messages = db::list_messages(&state.pool, &conversation_id)
        .await?
        .into_iter()
        .map(MessageDto::from)
        .collect();
    Ok(Json(MessageListResponse { messages }))
}

async fn send_message(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
    payload: Result<Json<SendMessageRequest>, JsonRejection>,
) -> Result<Json<SendMessageResponse>, AppError> {
    let Json(payload) = payload.map_err(AppError::invalid_json)?;
    let content = validate_message(payload.content)?;
    let turn = prepare_provider_turn(&state, &conversation_id, &content).await?;

    let assistant_content = state.chat_provider.complete(turn.provider_messages).await?;
    let (conversation, user_message) =
        persist_user_turn(&state, &conversation_id, &turn.conversation, &content).await?;
    let assistant_message = db::create_message(
        &state.pool,
        &conversation_id,
        "assistant",
        &assistant_content,
    )
    .await?;

    Ok(Json(SendMessageResponse {
        conversation: conversation.into(),
        user_message: user_message.into(),
        assistant_message: assistant_message.into(),
    }))
}

async fn stream_message(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
    payload: Result<Json<SendMessageRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(payload) = payload.map_err(AppError::invalid_json)?;
    let content = validate_message(payload.content)?;
    let turn = prepare_provider_turn(&state, &conversation_id, &content).await?;
    let provider_stream = state.chat_provider.stream(turn.provider_messages).await?;
    let (_conversation, user_message) =
        persist_user_turn(&state, &conversation_id, &turn.conversation, &content).await?;
    let pool = state.pool.clone();
    let user_event = StreamEvent::UserMessage {
        message: user_message.into(),
    };

    let stream = async_stream::stream! {
        yield ndjson(user_event);

        let mut assistant_content = String::new();
        tokio::pin!(provider_stream);

        while let Some(next) = provider_stream.next().await {
            match next {
                Ok(content) => {
                    assistant_content.push_str(&content);
                    yield ndjson(StreamEvent::Chunk { content });
                }
                Err(error) => {
                    yield ndjson(StreamEvent::Error {
                        code: "provider_error".to_string(),
                        message: error.public_message(),
                    });
                    return;
                }
            }
        }

        match db::create_message(&pool, &conversation_id, "assistant", &assistant_content).await {
            Ok(message) => {
                yield ndjson(StreamEvent::AssistantMessage { message: message.into() });
                yield ndjson(StreamEvent::Done);
            }
            Err(error) => {
                let detail = error.detail();
                yield ndjson(StreamEvent::Error {
                    code: detail.code,
                    message: detail.message,
                });
            }
        }
    };

    Ok((
        [(header::CONTENT_TYPE, "application/x-ndjson")],
        Body::from_stream(stream),
    )
        .into_response())
}

async fn ensure_conversation(
    state: &AppState,
    conversation_id: &str,
) -> Result<db::ConversationRecord, AppError> {
    db::get_conversation(&state.pool, conversation_id)
        .await?
        .ok_or_else(|| AppError::not_found("conversation not found"))
}

async fn prepare_provider_turn(
    state: &AppState,
    conversation_id: &str,
    content: &str,
) -> Result<PreparedProviderTurn, AppError> {
    let conversation = ensure_conversation(state, conversation_id).await?;
    let existing_messages = db::list_messages(&state.pool, conversation_id).await?;

    let mut provider_messages = existing_messages
        .into_iter()
        .map(|message| ProviderMessage {
            role: message.role,
            content: message.content,
        })
        .collect::<Vec<_>>();
    provider_messages.push(ProviderMessage {
        role: "user".to_string(),
        content: content.to_string(),
    });

    Ok(PreparedProviderTurn {
        conversation,
        provider_messages,
    })
}

async fn persist_user_turn(
    state: &AppState,
    conversation_id: &str,
    conversation: &db::ConversationRecord,
    content: &str,
) -> Result<(db::ConversationRecord, db::MessageRecord), AppError> {
    let conversation = db::maybe_set_conversation_title(&state.pool, conversation, content).await?;
    let user_message = db::create_message(&state.pool, conversation_id, "user", content).await?;
    Ok((conversation, user_message))
}

fn validate_title(title: Option<String>) -> Result<(String, bool), AppError> {
    let Some(title) = title else {
        return Ok(("New chat".to_string(), true));
    };
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::validation("title cannot be blank"));
    }
    if title.chars().count() > 120 {
        return Err(AppError::validation("title cannot exceed 120 characters"));
    }
    Ok((title.to_string(), false))
}

fn validate_message(content: String) -> Result<String, AppError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(AppError::validation("message content cannot be blank"));
    }
    if content.chars().count() > 8_000 {
        return Err(AppError::validation(
            "message content cannot exceed 8000 characters",
        ));
    }
    Ok(content.to_string())
}

fn ndjson(event: StreamEvent) -> Result<Bytes, Infallible> {
    let line = match serde_json::to_string(&event) {
        Ok(value) => format!("{value}\n"),
        Err(error) => format!(
            "{}\n",
            json!({
                "type": "error",
                "code": "serialization_error",
                "message": error.to_string()
            })
        ),
    };
    Ok(Bytes::from(line))
}

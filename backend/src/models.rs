use serde::{Deserialize, Serialize};

use crate::db::{ConversationRecord, MessageRecord};

#[derive(Clone, Debug, Serialize)]
pub struct ConversationDto {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ConversationRecord> for ConversationDto {
    fn from(value: ConversationRecord) -> Self {
        Self {
            id: value.id,
            title: value.title,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MessageDto {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl From<MessageRecord> for MessageDto {
    fn from(value: MessageRecord) -> Self {
        Self {
            id: value.id,
            conversation_id: value.conversation_id,
            role: value.role,
            content: value.content,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationDto>,
}

#[derive(Debug, Serialize)]
pub struct CreateConversationResponse {
    pub conversation: ConversationDto,
}

#[derive(Debug, Serialize)]
pub struct MessageListResponse {
    pub messages: Vec<MessageDto>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub conversation: ConversationDto,
    pub user_message: MessageDto,
    pub assistant_message: MessageDto,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    UserMessage { message: MessageDto },
    Chunk { content: String },
    AssistantMessage { message: MessageDto },
    Done,
    Error { code: String, message: String },
}

pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod openrouter;
pub mod routes;

pub use routes::{AppState, create_app};

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use futures_util::stream;
    use serde_json::{Value, json};
    use sqlx::sqlite::SqlitePoolOptions;
    use tower::ServiceExt;

    use crate::{
        db::{init_db, list_messages},
        openrouter::{ChatProvider, ChatProviderError, ProviderMessage, ProviderStream},
        routes::{AppState, create_app},
    };

    #[derive(Clone)]
    struct MockProvider {
        response: String,
    }

    #[async_trait]
    impl ChatProvider for MockProvider {
        async fn complete(
            &self,
            _messages: Vec<ProviderMessage>,
        ) -> Result<String, ChatProviderError> {
            Ok(self.response.clone())
        }

        async fn stream(
            &self,
            _messages: Vec<ProviderMessage>,
        ) -> Result<ProviderStream, ChatProviderError> {
            let chunks = vec![Ok("mock ".to_string()), Ok("stream".to_string())];
            Ok(Box::pin(stream::iter(chunks)))
        }
    }

    async fn test_app() -> Router {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        init_db(&pool).await.expect("initialize db");
        let state = AppState::new(
            pool,
            Arc::new(MockProvider {
                response: "mock assistant".to_string(),
            }),
        );
        create_app(state, "http://localhost:3000").expect("create app")
    }

    async fn read_json(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("json body")
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_json(response).await, json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn validation_errors_are_structured() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/conversations")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{ "title": "   " }"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = read_json(response).await;
        assert_eq!(json["error"]["code"], "validation_error");
        assert!(json["error"]["message"].as_str().unwrap().contains("title"));
    }

    #[tokio::test]
    async fn conversations_and_messages_persist() {
        let app = test_app().await;

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/conversations")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{ "title": "Persistence" }"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let created_json = read_json(created).await;
        let conversation_id = created_json["conversation"]["id"].as_str().unwrap();

        let listed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/conversations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(listed.status(), StatusCode::OK);
        let listed_json = read_json(listed).await;
        assert_eq!(listed_json["conversations"][0]["id"], conversation_id);

        let sent = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/conversations/{conversation_id}/messages"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{ "content": "Hello" }"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(sent.status(), StatusCode::OK);
        let sent_json = read_json(sent).await;
        assert_eq!(sent_json["user_message"]["content"], "Hello");
        assert_eq!(sent_json["assistant_message"]["content"], "mock assistant");

        let loaded = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/conversations/{conversation_id}/messages"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let loaded_json = read_json(loaded).await;
        assert_eq!(loaded_json["messages"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn mocked_provider_stream_persists_assistant_message() {
        let app = test_app().await;
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/conversations")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created_json = read_json(created).await;
        let conversation_id = created_json["conversation"]["id"].as_str().unwrap();

        let streamed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/conversations/{conversation_id}/messages/stream"
                    ))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{ "content": "Stream please" }"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(streamed.status(), StatusCode::OK);
        let body = to_bytes(streamed.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains(r#""type":"chunk""#));
        assert!(text.contains("mock stream"));

        let loaded = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/conversations/{conversation_id}/messages"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let loaded_json = read_json(loaded).await;
        assert_eq!(loaded_json["messages"][1]["content"], "mock stream");
    }

    #[tokio::test]
    async fn repository_returns_persisted_messages() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        init_db(&pool).await.unwrap();
        let conversation = crate::db::create_conversation(&pool, "Repository")
            .await
            .unwrap();
        crate::db::create_message(&pool, &conversation.id, "user", "Hello")
            .await
            .unwrap();

        let messages = list_messages(&pool, &conversation.id).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello");
    }
}

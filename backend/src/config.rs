use std::{env, net::SocketAddr};

use crate::error::AppError;

pub const DEFAULT_OPENROUTER_MODEL: &str = "nvidia/nemotron-3-super-120b-a12b:free";

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub openrouter_api_key: Option<String>,
    pub openrouter_model: String,
    pub openrouter_base_url: String,
    pub frontend_origin: String,
    pub bind_addr: SocketAddr,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        dotenvy::dotenv().ok();

        let bind_addr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse()
            .map_err(|_| AppError::config("BIND_ADDR must be a valid socket address"))?;

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://backend/gpt-copy-v4.sqlite3".to_string()),
            openrouter_api_key: env::var("OPENROUTER_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            openrouter_model: env::var("OPENROUTER_MODEL")
                .unwrap_or_else(|_| DEFAULT_OPENROUTER_MODEL.to_string()),
            openrouter_base_url: env::var("OPENROUTER_BASE_URL")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string()),
            frontend_origin: env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            bind_addr,
        })
    }
}

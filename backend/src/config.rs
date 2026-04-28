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

        let bind_addr: SocketAddr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse()
            .map_err(|_| AppError::config("BIND_ADDR must be a valid socket address"))?;

        if !bind_addr.ip().is_loopback() {
            return Err(AppError::config(
                "BIND_ADDR must use a loopback address for this local-only backend",
            ));
        }

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://gpt-copy-v4.sqlite3".to_string()),
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

#[cfg(test)]
mod tests {
    use std::{
        env,
        sync::{Mutex, MutexGuard},
    };

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvSnapshot {
        values: Vec<(&'static str, Option<String>)>,
    }

    impl EnvSnapshot {
        fn capture(keys: &[&'static str]) -> Self {
            Self {
                values: keys.iter().map(|key| (*key, env::var(key).ok())).collect(),
            }
        }
    }

    impl Drop for EnvSnapshot {
        fn drop(&mut self) {
            for (key, value) in &self.values {
                match value {
                    Some(value) => set_env(key, value),
                    None => remove_env(key),
                }
            }
        }
    }

    fn clean_env() -> (MutexGuard<'static, ()>, EnvSnapshot) {
        let guard = ENV_LOCK.lock().expect("env lock");
        let keys = [
            "DATABASE_URL",
            "OPENROUTER_API_KEY",
            "OPENROUTER_MODEL",
            "OPENROUTER_BASE_URL",
            "FRONTEND_ORIGIN",
            "BIND_ADDR",
        ];
        let snapshot = EnvSnapshot::capture(&keys);
        for key in keys {
            remove_env(key);
        }
        (guard, snapshot)
    }

    fn set_env(key: &str, value: &str) {
        // SAFETY: these tests serialize all process environment mutation with ENV_LOCK.
        unsafe {
            env::set_var(key, value);
        }
    }

    fn remove_env(key: &str) {
        // SAFETY: these tests serialize all process environment mutation with ENV_LOCK.
        unsafe {
            env::remove_var(key);
        }
    }

    #[test]
    fn default_database_url_matches_documented_backend_cwd() {
        let (_guard, _snapshot) = clean_env();

        let config = AppConfig::from_env().expect("default config");

        assert_eq!(config.database_url, "sqlite://gpt-copy-v4.sqlite3");
        assert!(config.bind_addr.ip().is_loopback());
    }

    #[test]
    fn non_loopback_bind_is_rejected_by_default() {
        let (_guard, _snapshot) = clean_env();
        set_env("BIND_ADDR", "0.0.0.0:8080");

        let error = AppConfig::from_env().expect_err("non-loopback bind should fail");

        assert!(error.to_string().contains("loopback"));
    }
}

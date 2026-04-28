use std::sync::Arc;

use gpt_copy_v4_backend::{
    AppState, config::AppConfig, create_app, db, openrouter::OpenRouterClient,
};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;
    let pool = db::create_pool(&config.database_url).await?;
    db::init_db(&pool).await?;

    let provider = Arc::new(OpenRouterClient::new(
        config.openrouter_api_key.clone(),
        config.openrouter_model.clone(),
        config.openrouter_base_url.clone(),
    ));
    let app = create_app(AppState::new(pool, provider), &config.frontend_origin)?;

    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!("backend listening on http://{}", config.bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

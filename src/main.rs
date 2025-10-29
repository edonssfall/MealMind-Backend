use mealmind::app;
use mealmind::state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "mealmind=debug,axum=info,tower_http=info".to_string());
    let json_logs = std::env::var("LOG_FORMAT")
        .map(|v| v == "json")
        .unwrap_or(false);

    if json_logs {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

    let app_state = state::AppState::init().await?;

    // Run migrations if present
    if let Err(e) = sqlx::migrate!("./migrations").run(&app_state.db).await {
        tracing::warn!(error = %e, "migrations folder not found or migration failed; continuing");
    }

    let app = app::build_app(app_state);

    app::serve(app).await
}

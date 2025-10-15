use std::net::SocketAddr;

use axum::{routing::{get}, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod routes;
mod auth;

use crate::routes::{auth::auth_routes, me::me_route};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "mealmind=debug,axum=info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = db::AppState::init().await?;

    // Run migrations if present
    if sqlx::migrate!("./migrations").run(&app_state.db).await.is_err() {
        tracing::warn!("migrations folder not found or migration failed; continuing");
    }

    let app = Router::new()
        .merge(auth_routes())
        .route("/me", get(me_route))
        .with_state(app_state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!(
        "{}:{}",
        std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
        std::env::var("APP_PORT").unwrap_or_else(|_| "8080".into())
    )
    .parse()?;

    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

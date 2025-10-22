use std::net::SocketAddr;

use axum::{routing::get, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod auth;
mod config;
mod db;
mod routes;

use crate::routes::{auth::auth_routes, me::me_route};

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

    let app_state = db::AppState::init().await?;

    // Run migrations if present
    if let Err(e) = sqlx::migrate!("./migrations").run(&app_state.db).await {
        tracing::warn!(error = %e, "migrations folder not found or migration failed; continuing");
    }

    let app = Router::new()
        .merge(auth_routes())
        .route("/me", get(me_route))
        .with_state(app_state)
        .layer(CorsLayer::permissive())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &axum::http::Request<_>| {
                    let method = req.method().clone();
                    let uri = req.uri().clone();
                    tracing::info_span!("http_request", %method, uri = %uri)
                })
                .on_response(
                    |res: &axum::http::Response<_>,
                     _latency: std::time::Duration,
                     span: &tracing::Span| {
                        let status = res.status();
                        span.record("status", tracing::field::display(status));
                        if status.is_server_error() {
                            tracing::error!(%status, "response");
                        } else {
                            tracing::info!(%status, "response");
                        }
                    },
                ),
        );

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

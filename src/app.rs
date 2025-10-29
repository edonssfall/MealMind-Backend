use std::net::SocketAddr;
use axum::{Router, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use crate::db::AppState;
use crate::{auth, meals};

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .nest("/api/v1",
              Router::new()
                  .merge(auth::router())
                  .merge(meals::router())
                  .route("/health", get(|| async { "ok" }))
        )
        .with_state(state)
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
        )
}

pub async fn serve(app: Router) -> anyhow::Result<()> {
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
use crate::state::AppState;
use axum::Router;

mod dto;
pub(crate) mod extractors;
pub mod handlers;
pub mod repo;
mod repo_types;
pub mod services;

/// Combines all auth-related routes (register, login, refresh, me).
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(handlers::auth_routes())
        .merge(handlers::me_routes())
}

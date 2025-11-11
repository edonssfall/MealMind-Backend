use crate::state::AppState;
use axum::Router;

mod claims;
mod dto;
pub(crate) mod extractors;
pub mod handlers;
pub mod repo;
pub mod services;
mod repo_types;

/// Combines all auth-related routes (register, login, refresh, me).
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(handlers::auth_routes())
        .merge(handlers::me_routes())
}

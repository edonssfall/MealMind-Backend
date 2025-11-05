use crate::state::AppState;
use axum::Router;

mod dto;
pub mod handlers;
pub mod repo;
pub mod services;
pub(crate) mod extractors;
mod claims;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(handlers::auth_routes())
        .merge(handlers::me_routes())
}

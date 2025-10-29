mod dto;
pub mod handlers;
mod repo;
mod services;

use crate::state::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(handlers::read_routes())
        .merge(handlers::write_routes())
}

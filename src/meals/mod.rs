mod dto;
pub mod handlers;
mod repo;
mod repo_types;
mod services;

use crate::state::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new().merge(handlers::meals_routes())
}

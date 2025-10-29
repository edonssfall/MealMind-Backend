pub mod handlers;
mod service;
mod repo;
mod dto;

use axum::Router;
use crate::db::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // список/детали — можешь подключить свои уже существующие list/get
        .merge(handlers::read_router())
        // создание meal с фото (multipart) и альтернатива base64
        .merge(handlers::write_router())
}

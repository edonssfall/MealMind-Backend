use axum::{extract::{FromRef, State}, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use regex::Regex;

use crate::{auth::{jwt::JwtKeys, password}, db::{AppState, User}};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: PublicUser,
}

#[derive(Debug, Serialize)]
pub struct PublicUser {
    pub id: uuid::Uuid,
    pub email: String,
}

fn is_valid_email(email: &str) -> bool {
    lazy_static! {
        static ref EMAIL_RE: Regex = Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
    }
    EMAIL_RE.is_match(email)
}

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}

pub async fn register(
    State(state): State<AppState>,
    Json(mut payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    payload.email = payload.email.trim().to_lowercase();

    if !is_valid_email(&payload.email) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid email".into()));
    }

    if payload.password.len() < 8 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Password too short".into()));
    }

    // Ensure email is not taken
    if let Ok(Some(_)) = User::find_by_email(&state.db, &payload.email).await {
        return Err((axum::http::StatusCode::CONFLICT, "Email already registered".into()));
    }

    let hash = password::hash_password(&payload.password)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = User::create(&state.db, &payload.email, &hash)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse { token: String::new(), user: PublicUser { id: user.id, email: user.email } }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(mut payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    payload.email = payload.email.trim().to_lowercase();

    if !is_valid_email(&payload.email) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid email".into()));
    }

    let user = User::find_by_email(&state.db, &payload.email)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    let ok = password::verify_password(&payload.password, &user.password_hash)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !ok {
        return Err((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials".into()));
    }

    let keys = JwtKeys::from_ref(&state);
    let token = keys
        .sign(user.id)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse { token, user: PublicUser { id: user.id, email: user.email } }))
}

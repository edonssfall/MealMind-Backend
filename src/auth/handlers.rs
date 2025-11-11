use axum::{
    extract::{FromRef, State},
    routing::{get, post},
    Json, Router,
};
use tracing::{info, instrument};

use crate::{
    auth::{
        dto::{AuthResponse, LoginRequest, PublicUser, RefreshRequest, RegisterRequest},
        extractors::AuthUser,
        repo_types::User,
        services::{hash_password, is_valid_email, verify_password, JwtKeys},
    },
    state::AppState,
};

/// Auth endpoints: register, login, refresh
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
}

/// Protected user endpoint
pub fn me_routes() -> Router<AppState> {
    Router::new().route("/me", get(get_me))
}

#[instrument(skip(state, payload))]
pub async fn register(
    State(state): State<AppState>,
    Json(mut payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    payload.email = payload.email.trim().to_lowercase();

    // Validate input
    if !is_valid_email(&payload.email) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid email".into()));
    }
    if payload.password.len() < 8 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Password too short".into(),
        ));
    }

    // Check if email already exists
    if let Ok(Some(_)) = User::find_by_email(&state.db, &payload.email).await {
        return Err((
            axum::http::StatusCode::CONFLICT,
            "Email already registered".into(),
        ));
    }

    // Hash and store user
    let hash = hash_password(&payload.password)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let user = User::create(&state.db, &payload.email, &hash)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Generate JWT pair
    let keys = JwtKeys::from_ref(&state);
    let access_token = keys
        .sign_access(user.id)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let refresh_token = keys
        .sign_refresh(user.id)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!(user_id = %user.id, email = %user.email, "user registered");

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: PublicUser {
            id: user.id,
            email: user.email,
        },
    }))
}

#[instrument(skip(state, payload))]
pub async fn login(
    State(state): State<AppState>,
    Json(mut payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    payload.email = payload.email.trim().to_lowercase();

    // Check user exists
    let user = match User::find_by_email(&state.db, &payload.email).await {
        Ok(Some(u)) => u,
        _ => {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                "Invalid credentials".into(),
            ))
        }
    };

    // Verify password
    let ok = verify_password(&payload.password, &user.password_hash)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !ok {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            "Invalid credentials".into(),
        ));
    }

    // Generate JWT pair
    let keys = JwtKeys::from_ref(&state);
    let access_token = keys
        .sign_access(user.id)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let refresh_token = keys
        .sign_refresh(user.id)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!(user_id = %user.id, email = %user.email, "user logged in");

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: PublicUser {
            id: user.id,
            email: user.email,
        },
    }))
}

#[instrument(skip(state, payload))]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    let keys = JwtKeys::from_ref(&state);
    let claims = keys.verify_refresh(&payload.refresh_token).map_err(|_| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            "Invalid or expired token".into(),
        )
    })?;

    // New tokens
    let access_token = keys
        .sign_access(claims.sub)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let refresh_token = keys
        .sign_refresh(claims.sub)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get user
    let user = sqlx::query_as::<_, User>(
        r#"SELECT id, email, password_hash, created_at FROM users WHERE id = $1"#,
    )
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    .map_err(|_| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            "User not found".into(),
        )
    })?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: PublicUser {
            id: user.id,
            email: user.email,
        },
    }))
}

#[instrument(skip(state))]
pub async fn get_me(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<PublicUser>, (axum::http::StatusCode, String)> {
    let user = sqlx::query_as::<_, User>(
        r#"SELECT id, email, password_hash, created_at FROM users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            "User not found".into(),
        )
    })?;

    Ok(Json(PublicUser {
        id: user.id,
        email: user.email,
    }))
}

// -------------------- Tests --------------------

#[cfg(test)]
mod me_tests {
    use super::*;

    #[test]
    fn test_me_response_serialization() {
        let response = PublicUser {
            id: uuid::Uuid::new_v4(),
            email: "test@example.com".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("id"));
    }
}

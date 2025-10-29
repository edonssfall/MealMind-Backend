use axum::{
    extract::{FromRef, State},
    routing::{get, post},
    Json, Router,
};
use tracing::{error, info, instrument, warn};

use crate::{
    auth::{
        dto::{AuthResponse, LoginRequest, PublicUser, RefreshRequest, RegisterRequest},
        repo::User,
        services::{hash_password, is_valid_email, verify_password, AuthUser, JwtKeys},
    },
    state::AppState,
};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
}

pub fn me_routes() -> Router<AppState> {
    Router::new().route("/me", get(get_me))
}

#[instrument(skip(state, payload))]
pub async fn register(
    State(state): State<AppState>,
    Json(mut payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    payload.email = payload.email.trim().to_lowercase();

    if !is_valid_email(&payload.email) {
        warn!(email = %payload.email, "invalid email");
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid email".into()));
    }

    if payload.password.len() < 8 {
        warn!("password too short");
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Password too short".into(),
        ));
    }

    // Ensure email is not taken
    if let Ok(Some(_)) = User::find_by_email(&state.db, &payload.email).await {
        warn!(email = %payload.email, "email already registered");
        return Err((
            axum::http::StatusCode::CONFLICT,
            "Email already registered".into(),
        ));
    }

    let hash = match hash_password(&payload.password) {
        Ok(h) => h,
        Err(e) => {
            error!(error = %e, "hash_password failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

    let user = match User::create(&state.db, &payload.email, &hash).await {
        Ok(u) => u,
        Err(e) => {
            error!(error = %e, "create user failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

    let keys = JwtKeys::from_ref(&state);
    let access_token = match keys.sign_access(user.id) {
        Ok(t) => t,
        Err(e) => {
            error!(error = %e, "jwt sign access failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };
    let refresh_token = match keys.sign_refresh(user.id) {
        Ok(t) => t,
        Err(e) => {
            error!(error = %e, "jwt sign refresh failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

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

    if !is_valid_email(&payload.email) {
        warn!(email = %payload.email, "invalid email");
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid email".into()));
    }

    let user = match User::find_by_email(&state.db, &payload.email).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            warn!(email = %payload.email, "login unknown email");
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                "Invalid credentials".into(),
            ));
        }
        Err(e) => {
            error!(error = %e, "find_by_email failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

    let ok = match verify_password(&payload.password, &user.password_hash) {
        Ok(v) => v,
        Err(e) => {
            error!(error = %e, "verify_password failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

    if !ok {
        warn!(email = %payload.email, user_id = %user.id, "login invalid password");
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            "Invalid credentials".into(),
        ));
    }

    let keys = JwtKeys::from_ref(&state);
    let access_token = match keys.sign_access(user.id) {
        Ok(t) => t,
        Err(e) => {
            error!(error = %e, "jwt sign access failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };
    let refresh_token = match keys.sign_refresh(user.id) {
        Ok(t) => t,
        Err(e) => {
            error!(error = %e, "jwt sign refresh failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };

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
    let claims = keys
        .verify_refresh(&payload.refresh_token)
        .map_err(|e| (axum::http::StatusCode::UNAUTHORIZED, format!("{}", e)))?;

    // Issue new pair
    let access_token = keys
        .sign_access(claims.sub)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let refresh_token = keys
        .sign_refresh(claims.sub)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Load public user
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
    .map_err(|e| {
        error!(error = %e, user_id = %user_id, "user not found");
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

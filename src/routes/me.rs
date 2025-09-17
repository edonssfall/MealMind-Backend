use axum::{extract::State, Json};
use serde::Serialize;

use crate::{auth::jwt::AuthUser, db::{AppState, User}};

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: uuid::Uuid,
    pub email: String,
}

pub async fn me_route(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<MeResponse>, (axum::http::StatusCode, String)> {
    let user = sqlx::query_as::<_, User>(
        r#"SELECT id, email, password_hash, created_at FROM users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "User not found".into()))?;

    Ok(Json(MeResponse { id: user.id, email: user.email }))
}

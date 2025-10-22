use axum::{extract::State, Json};
use serde::Serialize;
use tracing::{error, instrument};

use crate::{
    auth::jwt::AuthUser,
    db::{AppState, User},
};

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: uuid::Uuid,
    pub email: String,
}

#[instrument(skip(state))]
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
    .map_err(|e| {
        error!(error = %e, user_id = %user_id, "user not found");
        (
            axum::http::StatusCode::UNAUTHORIZED,
            "User not found".into(),
        )
    })?;

    Ok(Json(MeResponse {
        id: user.id,
        email: user.email,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_me_response_serialization() {
        let response = MeResponse {
            id: uuid::Uuid::new_v4(),
            email: "test@example.com".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("id"));
    }
}

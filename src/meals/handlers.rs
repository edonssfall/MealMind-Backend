use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::auth::extractors::AuthUser;
use crate::meals::{dto::*, repo, services};
use crate::state::AppState;

pub fn meals_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/meals",
            post(create_meal)
                .get(list_meals)
                .put(put_meal)
                .delete(delete_meal),
        )
        .route("/meals/:id", get(get_meal))
}

#[tracing::instrument(skip(st, req), fields(user_id = %user_id))]
async fn create_meal(
    State(st): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreatedMealRequest>,
) -> Result<Json<CreatedMealResponse>, (StatusCode, String)> {
    let resp = services::create_meal_with_images(&st, user_id, req)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "create_meal failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to create meal".into())
        })?;
    Ok(Json(resp))
}

#[tracing::instrument(skip(st), fields(user_id = %user_id, limit = p.limit, offset = p.offset))]
async fn list_meals(
    State(st): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<MealResponce>>, (StatusCode, String)> {
    let rows = repo::list_meals(&st.db, user_id, p.limit, p.offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "list_meals failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to list meals".into())
        })?;
    Ok(Json(rows))
}

#[tracing::instrument(skip(st), fields(user_id = %user_id, meal_id = %id))]
async fn get_meal(
    State(st): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<MealDetails>, (StatusCode, String)> {
    let m = repo::get_meal_details(&st.db, user_id, id)
        .await
        .map_err(|e| {
            // Если хочешь отличать 404:
            if let Some(sqlx::Error::RowNotFound) = e.downcast_ref::<sqlx::Error>() {
                return (StatusCode::NOT_FOUND, "meal not found".into());
            }
            tracing::error!(error = %e, "get_meal failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to get meal".into())
        })?;
    Ok(Json(m))
}

#[tracing::instrument(skip(st, req), fields(user_id = %user_id, meal_id = %req.id))]
async fn put_meal(
    State(st): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<PutMealRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::update_meal_full(&st.db, user_id, req.id, req.title, req.notes)
        .await
        .map_err(|e| {
            if let Some(sqlx::Error::RowNotFound) = e.downcast_ref::<sqlx::Error>() {
                return (StatusCode::NOT_FOUND, "meal not found".into());
            }
            tracing::error!(error = %e, "put_meal failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to update meal".into())
        })?;
    Ok(StatusCode::NO_CONTENT)
}

#[tracing::instrument(skip(st, req), fields(user_id = %user_id, meal_id = %req.id))]
async fn delete_meal(
    State(st): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<DeleteMealRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::unlink_meal_from_user(&st.db, user_id, req.id)
        .await
        .map_err(|e| {
            if let Some(sqlx::Error::RowNotFound) = e.downcast_ref::<sqlx::Error>() {
                return (StatusCode::NOT_FOUND, "meal not found".into());
            }
            tracing::error!(error = %e, "delete_meal (unlink) failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to unlink meal".into())
        })?;
    Ok(StatusCode::NO_CONTENT)
}

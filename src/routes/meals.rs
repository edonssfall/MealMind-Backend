use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
use uuid::Uuid;

use crate::{
    auth::jwt::AuthUser,
    db::{AppState, Meal, MealNutrition},
};

#[derive(Debug, Serialize)]
pub struct MealListItem {
    pub id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct MealDetails {
    pub id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub created_at: time::OffsetDateTime,
    pub nutrition: Option<MealNutrition>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}
fn default_limit() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
pub struct CreateMealBody {
    pub title: Option<String>,
    pub notes: Option<String>,
}

pub fn meals_routes() -> Router<AppState> {
    Router::new()
        .route("/meals", get(list_meals))
        .route("/meals/:id", get(get_meal))
}

#[instrument(skip(state))]
pub async fn list_meals(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<MealListItem>>, (axum::http::StatusCode, String)> {
    let meals = Meal::list_by_user(&state.db, user_id, p.limit, p.offset)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let items = meals
        .into_iter()
        .map(|m| MealListItem {
            id: m.id,
            title: m.title,
            notes: m.notes,
            created_at: m.created_at,
        })
        .collect();
    Ok(Json(items))
}

#[instrument(skip(state))]
pub async fn get_meal(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<MealDetails>, (axum::http::StatusCode, String)> {
    let (meal, nutrition) = match Meal::get_with_nutrition(&state.db, user_id, id).await {
        Ok(tuple) => tuple,
        Err(e) => {
            error!(error = %e, %user_id, %id, "meal not found");
            return Err((axum::http::StatusCode::NOT_FOUND, "Meal not found".into()));
        }
    };
    Ok(Json(MealDetails {
        id: meal.id,
        title: meal.title,
        notes: meal.notes,
        created_at: meal.created_at,
        nutrition,
    }))
}

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use tracing::{error, instrument};
use uuid::Uuid;

use crate::{
    auth::jwt::AuthUser,
    db::{AppState, Meal},
};

use super::dto::{Pagination, MealListItem, MealDetails, CreatedMealResponse, CreateMealBase64};
use super::service::{create_meal_with_photos, UploadItem};

// --- public routers ---

pub fn read_router() -> Router<AppState> {
    Router::new()
        .route("/meals", get(list_meals))
        .route("/meals/:id", get(get_meal))
        .route("/meals/:id/photo", get(get_presigned_photo)) // отдаём 302 на url первой фотки
}

pub fn write_router() -> Router<AppState> {
    Router::new()
        .route("/meals", post(create_meal_multipart)) // multipart files[]
        .route("/meals/base64", post(create_meal_base64))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024)) // 20MB
}

// --- handlers ---

#[instrument(skip(state))]
pub async fn list_meals(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<MealListItem>>, (StatusCode, String)> {
    let meals = Meal::list_by_user(&state.db, user_id, p.limit, p.offset)
        .await
        .map_err(internal)?;
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
) -> Result<Json<MealDetails>, (StatusCode, String)> {
    match Meal::get_with_nutrition(&state.db, user_id, id).await {
        Ok((meal, nutrition)) => Ok(Json(MealDetails {
            id: meal.id,
            title: meal.title,
            notes: meal.notes,
            created_at: meal.created_at,
            nutrition,
        })),
        Err(e) => {
            error!(error = %e, %user_id, %id, "get_meal failed");
            Err((StatusCode::NOT_FOUND, "Meal not found".into()))
        }
    }
}

/// POST /meals (multipart)
/// Поле: files[] (несколько файлов), опционально title/notes игнорируем (meal пустой)
#[instrument(skip(state, mp))]
pub async fn create_meal_multipart(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    mut mp: Multipart,
) -> Result<(StatusCode, HeaderMap, Json<CreatedMealResponse>), (StatusCode, String)> {
    let mut files: Vec<UploadItem<'_>> = Vec::new();
    while let Ok(Some(field)) = mp.next_field().await {
        let name = field.name().map(|s| s.to_string());
        if name.as_deref() == Some("files") || name.as_deref() == Some("files[]") {
            let content_type = field
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "application/octet-stream".into());
            let data = field.bytes().await.map_err(internal)?;
            files.push(UploadItem { body: data, content_type: Box::leak(content_type.into_boxed_str()) });
        }
    }
    if files.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "files[] is required".into()));
    }

    let (meal_id, created_at, photo_ids) =
        create_meal_with_photos(&state, user_id, files).await.map_err(internal)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::LOCATION,
        format!("/meals/{}", meal_id).parse().unwrap(),
    );

    Ok((
        StatusCode::CREATED,
        headers,
        Json(CreatedMealResponse { id: meal_id, created_at, photo_ids }),
    ))
}

/// POST /meals/base64 { images_b64: ["...","..."], content_type?: "image/jpeg" }
#[instrument(skip(state, body))]
pub async fn create_meal_base64(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(body): Json<CreateMealBase64>,
) -> Result<(StatusCode, HeaderMap, Json<CreatedMealResponse>), (StatusCode, String)> {
    if body.images_b64.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "images_b64 is required".into()));
    }
    let ct = body.content_type.as_deref().unwrap_or("application/octet-stream");

    let mut files = Vec::with_capacity(body.images_b64.len());
    for b64 in body.images_b64 {
        let bytes = base64::decode(b64).map_err(|_| (StatusCode::BAD_REQUEST, "invalid base64".into()))?;
        files.push(UploadItem { body: Bytes::from(bytes), content_type: ct });
    }

    let (meal_id, created_at, photo_ids) =
        create_meal_with_photos(&state, user_id, files).await.map_err(internal)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::LOCATION,
        format!("/meals/{}", meal_id).parse().unwrap(),
    );

    Ok((
        StatusCode::CREATED,
        headers,
        Json(CreatedMealResponse { id: meal_id, created_at, photo_ids }),
    ))
}

/// 302 → presigned url первой фотки (если нужна галерея — сделаем list)
#[instrument(skip(state))]
pub async fn get_presigned_photo(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let row = sqlx::query!(
        r#"
        SELECT p.s3_key
        FROM photos p
        JOIN meals m ON m.id = p.meal_id
        WHERE m.id = $1 AND m.user_id = $2
        ORDER BY p.created_at ASC
        LIMIT 1
        "#,
        id, user_id
    )
        .fetch_optional(&state.db)
        .await;

    let Some(row) = match row {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    } else {
        return (StatusCode::NOT_FOUND, "Photo not found").into_response();
    };

    let Ok(url) = state.storage.presign_get(&row.s3_key, 600).await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "presign failed").into_response();
    };

    Redirect::temporary(&url).into_response()
}

fn internal<E: std::error::Error>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

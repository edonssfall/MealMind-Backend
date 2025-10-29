use bytes::Bytes;
use crate::images::services::{upload_and_link_images, UploadItem};
use crate::meals::dto::{CreatedMealRequest, CreatedMealResponse};

pub async fn create_meal_bytes(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(body): Json<CreatedMealRequest>,
) -> Result<(StatusCode, HeaderMap, Json<CreatedMealResponse>), (StatusCode, String)> {
    if body.images.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "images must be non-empty".into()));
    }

    // 1) создаём пустой meal c заранее заданным UUID
    let meal_id = Uuid::new_v4();
    let meal = sqlx::query_as::<_, Meal>(
        r#"INSERT INTO meals (id, user_id)
           VALUES ($1, $2)
           RETURNING id, user_id, title, notes, created_at"#,
    )
        .bind(meal_id)
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;

    // 2) подготовим UploadItem[]
    // если у тебя один общий content-type — прокинь его; иначе возьми "application/octet-stream"
    let ct = "application/octet-stream";
    let files: Vec<UploadItem> = body.images.into_iter()
        .map(|buf| UploadItem {
            body: Bytes::from(buf.into_vec()),
            content_type: ct,
        })
        .collect();

    // 3) зальём и привяжем к meal
    let photo_ids = upload_and_link_images(&state, user_id, meal_id, files)
        .await
        .map_err(internal)?;

    // 4) ответ
    let mut headers = HeaderMap::new();
    headers.insert(axum::http::header::LOCATION, format!("/meals/{}", meal_id).parse().unwrap());

    Ok((
        StatusCode::CREATED,
        headers,
        Json(CreatedMealResponse {
            id: meal.id,
            created_at: meal.created_at,
            photo_ids,
        }),
    ))
}

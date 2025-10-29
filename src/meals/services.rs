use anyhow::Context;
use bytes::Bytes;
use uuid::Uuid;
use crate::db::AppState;
use super::repo;

pub struct UploadItem<'a> {
    pub body: Bytes,
    pub content_type: &'a str,
}

/// Загружает несколько файлов в MinIO, затем в одной транзакции создаёт meal и записи photos.
/// При ошибке вставки в БД пытается удалить уже загруженные объекты (best-effort).
pub async fn create_meal_with_photos(
    state: &crate::db::AppState,
    user_id: Uuid,
    files: Vec<UploadItem<'_>>,
) -> anyhow::Result<(uuid::Uuid, time::OffsetDateTime, Vec<uuid::Uuid>)> {
    anyhow::ensure!(!files.is_empty(), "no files provided");

    // 1) Сначала грузим все объекты в хранилище и собираем ключи
    struct StoredObj {
        key: String,
        photo_id: Uuid,
    }
    let mut stored: Vec<StoredObj> = Vec::with_capacity(files.len());

    // meal_id заранее
    let meal_id = Uuid::new_v4();

    for file in &files {
        let photo_id = Uuid::new_v4();
        let ext = ext_from_mime(file.content_type).unwrap_or("bin");
        let key = format!("meals/{}/{}-{}.{}", user_id, meal_id, photo_id, ext);

        state.storage
            .put_object(&key, file.body.clone(), file.content_type)
            .await
            .with_context(|| format!("put_object {}", key))?;

        stored.push(StoredObj { key, photo_id });
    }

    // 2) Пишем в БД в транзакции
    let mut tx = state.db.begin().await.context("begin tx")?;

    let meal = repo::insert_meal_tx(&mut tx, meal_id, user_id).await?;

    for s in &stored {
        repo::insert_photo_tx(&mut tx, s.photo_id, user_id, meal_id, &s.key, None, "uploaded").await?;
    }

    tx.commit().await.context("commit tx")?;

    let photo_ids = stored.into_iter().map(|s| s.photo_id).collect();
    Ok((meal.id, meal.created_at, photo_ids))
}

fn ext_from_mime(ct: &str) -> Option<&'static str> {
    match ct {
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/heic" => Some("heic"),
        _ => None,
    }
}

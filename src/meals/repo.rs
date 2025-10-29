use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use crate::db::Meal;

pub async fn insert_meal_tx(
    tx: &mut Transaction<'_, Postgres>,
    meal_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<Meal> {
    let meal = sqlx::query_as::<_, Meal>(
        r#"
        INSERT INTO meals (id, user_id)
        VALUES ($1, $2)
        RETURNING id, user_id, title, notes, created_at
        "#,
    )
        .bind(meal_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .context("insert meal")?;

    Ok(meal)
}

pub async fn insert_photo_tx(
    tx: &mut Transaction<'_, Postgres>,
    photo_id: Uuid,
    user_id: Uuid,
    meal_id: Uuid,
    s3_key: &str,
    taken_at: Option<time::OffsetDateTime>,
    content_status: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO photos (id, user_id, meal_id, s3_key, taken_at, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        photo_id, user_id, meal_id, s3_key, taken_at, content_status
    )
        .execute(&mut *tx)
        .await
        .context("insert photo")?;

    Ok(())
}

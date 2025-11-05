use anyhow::Context;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug)]
pub struct PhotoRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub meal_id: Option<Uuid>,
    pub s3_key: String,
    pub created_at: OffsetDateTime,
}

pub async fn insert_photo_tx(
    tx: &mut Transaction<'_, Postgres>,
    photo_id: Uuid,
    meal_id: Option<Uuid>,
    s3_key: &str,
) -> anyhow::Result<()> {
    tx.execute(
        sqlx::query(
            r#"
            INSERT INTO photos (id, meal_id, s3_key, status)
            VALUES ($1, $2, $3, $4, 'uploaded')
            "#,
        )
        .bind(photo_id)
        .bind(meal_id) // Option<Uuid> → NULL ок
        .bind(s3_key),
    )
    .await
    .context("insert photo")?;

    Ok(())
}

pub async fn list_photo_ids_by_meal(
    db: &PgPool,
    meal_id: Uuid,
) -> anyhow::Result<Vec<(Uuid, String)>> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT id, s3_key
        FROM photos
        WHERE meal_id = $2
        ORDER BY created_at ASC
        "#,
    )
    .bind(meal_id)
    .fetch_all(db)
    .await
    .context("list photos by meal")?;

    Ok(rows)
}

pub async fn get_first_photo_by_meal(
    db: &PgPool,
    meal_id: Uuid,
) -> anyhow::Result<Option<(Uuid, String)>> {
    let row = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT id, s3_key
        FROM photos
        WHERE meal_id = $2
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .bind(meal_id)
    .fetch_optional(db)
    .await
    .context("get first photo by meal")?;

    Ok(row)
}

use anyhow::Context;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::meals::{
    dto::{MealDetails, MealResponce},
    repo_types::{ListMealRow, MealNutrition, MealRow, PhotoKeyRow},
};

/// Create a new meal inside a transaction.
pub async fn create_meal_tx(
    tx: &mut PgConnection,
    user_id: Uuid,
) -> anyhow::Result<(Uuid, OffsetDateTime)> {
    #[derive(sqlx::FromRow)]
    struct InsertRow {
        id: Uuid,
        created_at: OffsetDateTime,
    }

    let rec = sqlx::query_as::<_, InsertRow>(
        r#"
        INSERT INTO meals (user_id)
        VALUES ($1)
        RETURNING id, created_at
        "#,
    )
    .bind(user_id)
    .fetch_one(tx.as_mut())
    .await
    .context("insert meal")?;

    Ok((rec.id, rec.created_at))
}

/// Update meal title and notes.
pub async fn update_meal_full(
    db: &PgPool,
    user_id: Uuid,
    meal_id: Uuid,
    title: Option<String>,
    notes: Option<String>,
) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        UPDATE meals
           SET title = $1, notes = $2
         WHERE id = $3 AND (user_id = $4 OR user_id IS NULL)
        "#,
    )
    .bind(title)
    .bind(notes)
    .bind(meal_id)
    .bind(user_id)
    .execute(db)
    .await
    .context("update meal")?
    .rows_affected();

    anyhow::ensure!(rows == 1, "meal not found or not accessible");
    Ok(())
}

/// Unlink a meal from its owner (soft delete).
pub async fn unlink_meal_from_user(
    db: &PgPool,
    user_id: Uuid,
    meal_id: Uuid,
) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        UPDATE meals
           SET user_id = NULL
         WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(meal_id)
    .bind(user_id)
    .execute(db)
    .await
    .context("unlink meal")?
    .rows_affected();

    anyhow::ensure!(rows == 1, "meal not found or already unlinked");
    Ok(())
}

/// List user meals with preview photos.
pub async fn list_meals(
    db: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<MealResponce>> {
    let rows = sqlx::query_as::<_, ListMealRow>(
        r#"
        SELECT m.id, m.title, m.created_at,
               COALESCE(
                 (SELECT array_agg(p.s3_key ORDER BY p.created_at ASC)
                    FROM photos p
                   WHERE p.meal_id = m.id), '{}'
               ) AS photos
          FROM meals m
         WHERE m.user_id = $1
         ORDER BY m.created_at DESC
         LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .context("list meals")?;

    Ok(rows
        .into_iter()
        .map(|r| MealResponce {
            id: r.id,
            title: r.title,
            created_at: r.created_at,
            photos: r.photos.unwrap_or_default(),
        })
        .collect())
}

/// Get full meal details including nutrition and photos.
pub async fn get_meal_details(
    db: &PgPool,
    user_id: Uuid,
    meal_id: Uuid,
) -> anyhow::Result<MealDetails> {
    // Load meal
    let m = sqlx::query_as::<_, MealRow>(
        r#"
        SELECT id, title, notes, created_at
          FROM meals
         WHERE id = $1 AND (user_id = $2 OR user_id IS NULL)
        "#,
    )
    .bind(meal_id)
    .bind(user_id)
    .fetch_one(db)
    .await
    .context("get meal")?;

    // Load nutrition (NUMERIC -> DOUBLE PRECISION for f64).
    let nutr: Option<MealNutrition> = sqlx::query_as::<_, MealNutrition>(
        r#"
        SELECT
               (total_calories_kcal)::DOUBLE PRECISION AS total_calories_kcal,
               (protein_g)::DOUBLE PRECISION          AS protein_g,
               (fat_g)::DOUBLE PRECISION              AS fat_g,
               (carbs_g)::DOUBLE PRECISION            AS carbs_g,
               (sodium_mg)::DOUBLE PRECISION          AS sodium_mg,
               (sugar_g)::DOUBLE PRECISION            AS sugar_g,
               (fiber_g)::DOUBLE PRECISION            AS fiber_g,
               micros,
               ai_raw,
               (global_score)::DOUBLE PRECISION       AS global_score,
               created_at
          FROM meal_nutrition
         WHERE meal_id = $1
        "#,
    )
    .bind(meal_id)
    .fetch_optional(db)
    .await
    .context("get nutrition")?;

    // Load photos
    let photos = sqlx::query_as::<_, PhotoKeyRow>(
        r#"
        SELECT s3_key
          FROM photos
         WHERE meal_id = $1
         ORDER BY created_at ASC
        "#,
    )
    .bind(meal_id)
    .fetch_all(db)
    .await
    .context("get meal photos")?
    .into_iter()
    .map(|r| r.s3_key)
    .collect::<Vec<_>>();

    Ok(MealDetails {
        id: m.id,
        title: m.title,
        notes: m.notes,
        created_at: m.created_at,
        nutrition: nutr,
        images: photos,
    })
}

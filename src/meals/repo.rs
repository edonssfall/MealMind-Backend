use anyhow::Context;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::meals::dto::{MealDetails, MealResponce};
use crate::meals::repo_types::MealNutrition;

#[derive(sqlx::FromRow)]
struct MealRow {
    id: Uuid,
    title: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ListMealRow {
    id: Uuid,
    title: Option<String>,
    created_at: OffsetDateTime,
    photos: Option<Vec<String>>,
}

#[derive(sqlx::FromRow)]
struct PhotoKeyRow {
    s3_key: String,
}

#[derive(sqlx::FromRow)]
struct NutritionF64Row {
//    meal_id: Uuid,
    total_calories_kcal: Option<f64>,
    protein_g: Option<f64>,
    fat_g: Option<f64>,
    carbs_g: Option<f64>,
    sodium_mg: Option<f64>,
    sugar_g: Option<f64>,
    fiber_g: Option<f64>,
    micros: serde_json::Value,
    ai_raw: serde_json::Value,
    global_score: Option<f64>,
    created_at: OffsetDateTime,
}

impl From<NutritionF64Row> for MealNutrition {
    fn from(r: NutritionF64Row) -> Self {
        Self {
            total_calories_kcal: r.total_calories_kcal,
            protein_g: r.protein_g,
            fat_g: r.fat_g,
            carbs_g: r.carbs_g,
            sodium_mg: r.sodium_mg,
            sugar_g: r.sugar_g,
            fiber_g: r.fiber_g,
            micros: r.micros,
            ai_raw: r.ai_raw,
            global_score: r.global_score,
            created_at: r.created_at,
        }
    }
}

pub async fn create_meal_tx(
    tx: &mut PgConnection,
    user_id: Uuid,
) -> anyhow::Result<(Uuid, OffsetDateTime)> {
    #[derive(sqlx::FromRow)]
    struct InsertRow { id: Uuid, created_at: OffsetDateTime }

    let rec = sqlx::query_as::<_, InsertRow>(r#"
        INSERT INTO meals (user_id)
        VALUES ($1)
        RETURNING id, created_at
    "#)
        .bind(user_id)
        .fetch_one(tx.as_mut())
        .await
        .context("insert meal")?;

    Ok((rec.id, rec.created_at))
}

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

pub async fn get_meal_details(
    db: &PgPool,
    user_id: Uuid,
    meal_id: Uuid,
) -> anyhow::Result<MealDetails> {
    // meal
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

    // nutrition → приводим NUMERIC к DOUBLE PRECISION (f64)
    let nutr = sqlx::query_as::<_, NutritionF64Row>(
        r#"
        SELECT meal_id,
               (total_calories_kcal)::DOUBLE PRECISION AS total_calories_kcal,
               (protein_g)::DOUBLE PRECISION          AS protein_g,
               (fat_g)::DOUBLE PRECISION              AS fat_g,
               (carbs_g)::DOUBLE PRECISION            AS carbs_g,
               (sodium_mg)::DOUBLE PRECISION          AS sodium_mg,
               (sugar_g)::DOUBLE PRECISION            AS sugar_g,
               (fiber_g)::DOUBLE PRECISION            AS fiber_g,
               micros, ai_raw,
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

    // photos
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
        nutrition: nutr.map(Into::into),
        images: photos,
    })
}

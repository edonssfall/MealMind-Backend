use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Meal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MealNutrition {
    pub meal_id: Uuid,
    pub total_calories_kcal: Option<sqlx::types::Decimal>,
    pub protein_g: Option<sqlx::types::Decimal>,
    pub fat_g: Option<sqlx::types::Decimal>,
    pub carbs_g: Option<sqlx::types::Decimal>,
    pub sodium_mg: Option<sqlx::types::Decimal>,
    pub sugar_g: Option<sqlx::types::Decimal>,
    pub fiber_g: Option<sqlx::types::Decimal>,
    pub micros: Option<serde_json::Value>,
    pub ai_raw: Option<serde_json::Value>,
    pub created_at: OffsetDateTime,
    pub global_score: Option<sqlx::types::Decimal>,
}

pub async fn list_by_user(
    db: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<Meal>> {
    let rows = sqlx::query_as::<_, Meal>(
        r#"
        SELECT id, user_id, title, notes, created_at
        FROM meals
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
    "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn get_with_nutrition(
    db: &PgPool,
    user_id: Uuid,
    meal_id: Uuid,
) -> anyhow::Result<(Meal, Option<MealNutrition>)> {
    let meal = sqlx::query_as::<_, Meal>(
        r#"
            SELECT id, user_id, title, notes, created_at
            FROM meals
            WHERE id = $1 AND user_id = $2
            "#,
    )
    .bind(meal_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;

    let nutrition = sqlx::query_as::<_, MealNutrition>(
        r#"
            SELECT meal_id, total_calories_kcal, protein_g, fat_g, carbs_g, sodium_mg,
                   sugar_g, fiber_g, micros, ai_raw, created_at, global_score
            FROM meal_nutrition
            WHERE meal_id = $1
            "#,
    )
    .bind(meal_id)
    .fetch_optional(db)
    .await?;

    Ok((meal, nutrition))
}

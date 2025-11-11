use serde::Serialize;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// Internal DB model for a single meal.
#[derive(FromRow)]
pub(crate) struct MealRow {
    pub(crate) id: Uuid,
    pub(crate) title: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) created_at: OffsetDateTime,
}

/// Compact meal row for list queries.
#[derive(FromRow)]
pub(crate) struct ListMealRow {
    pub(crate) id: Uuid,
    pub(crate) title: Option<String>,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) photos: Option<Vec<String>>,
}

/// Photo reference by S3 key.
#[derive(FromRow)]
pub(crate) struct PhotoKeyRow {
    pub(crate) s3_key: String,
}

/// Nutrition payload returned in API responses and loaded from DB.
#[derive(Debug, Serialize, FromRow)]
pub struct MealNutrition {
    pub total_calories_kcal: Option<f64>,
    pub protein_g: Option<f64>,
    pub fat_g: Option<f64>,
    pub carbs_g: Option<f64>,
    pub sodium_mg: Option<f64>,
    pub sugar_g: Option<f64>,
    pub fiber_g: Option<f64>,
    pub micros: serde_json::Value,
    pub ai_raw: serde_json::Value,
    pub global_score: Option<f64>,
    pub created_at: OffsetDateTime,
}

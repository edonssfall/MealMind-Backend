use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use crate::db::MealNutrition;

#[derive(Debug, Serialize)]
pub struct MealListItem {
    pub id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct MealDetails {
    pub id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub nutrition: Option<MealNutrition>,
}

#[derive(Debug, Serialize)]
pub struct CreatedMealResponse {
    pub id: Uuid,
    pub created_at: OffsetDateTime,
    pub photo_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}
fn default_limit() -> i64 { 20 }

#[derive(Debug, Deserialize)]
pub struct CreateMealBase64 {
    pub images_b64: Vec<String>,
    pub content_type: Option<String>,
}

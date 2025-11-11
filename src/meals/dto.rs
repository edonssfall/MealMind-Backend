use crate::meals::repo_types::MealNutrition;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Full meal data with nutrition and images.
#[derive(Debug, Serialize)]
pub struct MealDetails {
    pub id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub nutrition: Option<MealNutrition>,
    pub images: Vec<String>,
}

/// Request for creating a new meal with images.
#[derive(Debug, Deserialize)]
pub struct CreatedMealRequest {
    pub images: Vec<serde_bytes::ByteBuf>,
    #[serde(default)]
    pub content_types: Vec<String>, // optional MIME types
}

/// Request for updating an existing meal.
#[derive(Debug, Deserialize)]
pub struct PutMealRequest {
    pub id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
}

/// Request for deleting a meal.
#[derive(Debug, Deserialize)]
pub struct DeleteMealRequest {
    pub id: Uuid,
}

/// Basic meal info used in list responses.
#[derive(Debug, Serialize)]
pub struct MealResponce {
    pub id: Uuid,
    pub title: Option<String>,
    pub created_at: OffsetDateTime,
    pub photos: Vec<String>,
}

/// Response returned after meal creation.
#[derive(Debug, Serialize)]
pub struct CreatedMealResponse {
    pub id: Uuid,
    pub created_at: OffsetDateTime,
    pub images: Vec<Uuid>,
}

/// Pagination query params.
#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

use crate::meals::repo::MealNutrition;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct MealDetails {
    pub id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub nutrition: Option<MealNutrition>,
    pub images: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatedMealRequest {
    pub images: Vec<serde_bytes::ByteBuf>,
}

#[derive(Debug, Serialize)]
pub struct PutMealRequest {
    pub id: Uuid,
    pub title: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeleteMealRequest {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct MealResponce {
    pub id: Uuid,
    pub title: Option<String>,
    pub created_at: OffsetDateTime,
    pub photos: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatedMealResponse {
    pub id: Uuid,
    pub created_at: OffsetDateTime,
    pub images: Vec<Uuid>,
}

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

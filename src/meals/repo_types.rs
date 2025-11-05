use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug)]
pub struct MealNutritionRow {
//    pub meal_id: Uuid,
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

#[derive(Debug, Serialize)]
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

impl From<MealNutritionRow> for MealNutrition {
    fn from(r: MealNutritionRow) -> Self {
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

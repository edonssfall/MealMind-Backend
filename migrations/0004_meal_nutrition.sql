-- Meal nutrition table (1:1 with meals)
CREATE TABLE IF NOT EXISTS meal_nutrition (
    meal_id UUID PRIMARY KEY REFERENCES meals(id) ON DELETE CASCADE,
    total_calories_kcal NUMERIC(10,2),
    protein_g NUMERIC(10,2),
    fat_g NUMERIC(10,2),
    carbs_g NUMERIC(10,2),
    sodium_mg NUMERIC(10,2),
    sugar_g NUMERIC(10,2),
    fiber_g NUMERIC(10,2),
    micros JSONB,
    ai_raw JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);



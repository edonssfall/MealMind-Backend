-- Meals table
CREATE TABLE IF NOT EXISTS meals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    title VARCHAR(255),
    notes TEXT,
    global_score NUMERIC(5,2);
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Helpful indexes
CREATE INDEX IF NOT EXISTS idx_meals_user_id ON meals(user_id);
CREATE INDEX IF NOT EXISTS idx_meals_created_at ON meals(created_at);

-- Optional: basic check constraint for plausible range
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'meal_nutrition_global_score_range'
    ) THEN
ALTER TABLE meal_nutrition
    ADD CONSTRAINT meal_nutrition_global_score_range
        CHECK (global_score IS NULL OR (global_score >= 0 AND global_score <= 100));
END IF;
END$$;

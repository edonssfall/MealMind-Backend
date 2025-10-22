-- Add a global score to meal_nutrition to represent overall quality (0-100)
ALTER TABLE meal_nutrition
ADD COLUMN IF NOT EXISTS global_score NUMERIC(5,2);

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



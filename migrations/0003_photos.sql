-- Photos table (1:N relationship to meals via meal_id, optional)
CREATE TABLE IF NOT EXISTS photos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    meal_id UUID NOT NULL REFERENCES meals(id) ON DELETE RESTRICT,
    s3_key TEXT NOT NULL,
    status VARCHAR(255) NOT NULL DEFAULT 'uploaded',
    failure_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Helpful indexes
CREATE INDEX IF NOT EXISTS idx_photos_meal_id ON photos(meal_id);
CREATE INDEX IF NOT EXISTS idx_photos_created_at ON photos(created_at);



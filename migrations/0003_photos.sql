-- Photos table (1:N relationship to meals via meal_id, optional)
CREATE TABLE IF NOT EXISTS photos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    meal_id UUID REFERENCES meals(id) ON DELETE SET NULL,
    s3_key TEXT NOT NULL,
    taken_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'uploaded',
    failure_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Helpful indexes
CREATE INDEX IF NOT EXISTS idx_photos_user_id ON photos(user_id);
CREATE INDEX IF NOT EXISTS idx_photos_meal_id ON photos(meal_id);
CREATE INDEX IF NOT EXISTS idx_photos_created_at ON photos(created_at);



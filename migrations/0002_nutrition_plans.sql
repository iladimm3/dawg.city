-- Nutrition plans table
CREATE TABLE IF NOT EXISTS nutrition_plans (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dog_id                  UUID NOT NULL REFERENCES dogs(id) ON DELETE CASCADE,
    daily_calories          INTEGER NOT NULL,
    meals_per_day           INTEGER NOT NULL,
    portion_per_meal_grams  DOUBLE PRECISION NOT NULL,
    feeding_schedule        TEXT[] NOT NULL DEFAULT '{}',
    recommended_foods       TEXT[] NOT NULL DEFAULT '{}',
    foods_to_avoid          TEXT[] NOT NULL DEFAULT '{}',
    supplements             TEXT[] NOT NULL DEFAULT '{}',
    notes                   TEXT NOT NULL DEFAULT '',
    next_review_weeks       INTEGER NOT NULL,
    goal                    TEXT,
    food_brand              TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add photo_url to dogs table
ALTER TABLE dogs ADD COLUMN IF NOT EXISTS photo_url TEXT;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_nutrition_plans_dog ON nutrition_plans(dog_id);
CREATE INDEX IF NOT EXISTS idx_nutrition_plans_owner ON nutrition_plans(owner_id);

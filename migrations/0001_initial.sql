-- Users table
CREATE TABLE IF NOT EXISTS users (
    id              UUID PRIMARY KEY,
    google_sub      TEXT UNIQUE NOT NULL,
    email           TEXT UNIQUE NOT NULL,
    name            TEXT NOT NULL,
    avatar_url      TEXT,
    subscription_tier TEXT NOT NULL DEFAULT 'free', -- 'free' | 'pro'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dogs table
CREATE TABLE IF NOT EXISTS dogs (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    breed           TEXT NOT NULL,
    age_months      INTEGER NOT NULL,
    weight_kg       DOUBLE PRECISION NOT NULL,
    sex             TEXT NOT NULL CHECK (sex IN ('male', 'female')),
    neutered        BOOLEAN NOT NULL DEFAULT false,
    activity_level  TEXT NOT NULL CHECK (activity_level IN ('low', 'medium', 'high')),
    health_notes    TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Training session logs
CREATE TABLE IF NOT EXISTS training_logs (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dog_id          UUID NOT NULL REFERENCES dogs(id) ON DELETE CASCADE,
    session_title   TEXT NOT NULL,
    completed       BOOLEAN NOT NULL DEFAULT false,
    notes           TEXT,
    rating          INTEGER CHECK (rating BETWEEN 1 AND 5),
    logged_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_dogs_owner ON dogs(owner_id);
CREATE INDEX IF NOT EXISTS idx_training_logs_dog ON training_logs(dog_id);
CREATE INDEX IF NOT EXISTS idx_training_logs_owner ON training_logs(owner_id);

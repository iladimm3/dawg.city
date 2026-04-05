-- supabase/schema.sql
-- dawg.city v2 — Full schema for games portal + coin economy
-- Run in Supabase SQL editor or apply as a migration.
-- Requires: pgcrypto extension (enabled by default on Supabase)

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ── Users ──────────────────────────────────────────────────────────────────
-- Populated on first Discord OAuth login.
CREATE TABLE IF NOT EXISTS users (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  discord_id   TEXT UNIQUE NOT NULL,
  username     TEXT NOT NULL,
  avatar_url   TEXT,
  coins        INTEGER NOT NULL DEFAULT 0,
  xp           INTEGER NOT NULL DEFAULT 0,
  streak_days  INTEGER NOT NULL DEFAULT 0,
  last_seen    TIMESTAMPTZ,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── Sessions ───────────────────────────────────────────────────────────────
-- Opaque bearer tokens issued after Discord OAuth callback.
CREATE TABLE IF NOT EXISTS sessions (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token       TEXT UNIQUE NOT NULL,
  expires_at  TIMESTAMPTZ NOT NULL,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token);

-- ── Games ──────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS games (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  slug         TEXT UNIQUE NOT NULL,
  title        TEXT NOT NULL,
  description  TEXT,
  category     TEXT NOT NULL, -- 'action', 'puzzle', 'io', 'racing', etc.
  embed_url    TEXT NOT NULL,
  thumbnail    TEXT,
  tags         TEXT[],
  play_count   BIGINT NOT NULL DEFAULT 0,
  is_featured  BOOLEAN NOT NULL DEFAULT false,
  is_active    BOOLEAN NOT NULL DEFAULT true,
  added_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── Leaderboard ────────────────────────────────────────────────────────────
-- One row per user per game; updated only if the new score is higher.
CREATE TABLE IF NOT EXISTS leaderboard (
  id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  game_id   UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  score     BIGINT NOT NULL,
  posted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (game_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_leaderboard_game_score
  ON leaderboard(game_id, score DESC);

-- ── Coin transactions ──────────────────────────────────────────────────────
-- Append-only ledger. Never mutate users.coins directly.
-- The trigger below keeps users.coins in sync automatically.
CREATE TABLE IF NOT EXISTS coin_transactions (
  id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  amount     INTEGER NOT NULL,   -- positive = earned, negative = spent
  reason     TEXT NOT NULL,      -- 'playtime', 'streak', 'purchase', 'battle_pass'
  meta       JSONB,              -- optional context (game slug, item id, etc.)
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION sync_coin_balance()
RETURNS TRIGGER AS $$
BEGIN
  UPDATE users
  SET coins = coins + NEW.amount
  WHERE id = NEW.user_id;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS after_coin_transaction ON coin_transactions;
CREATE TRIGGER after_coin_transaction
AFTER INSERT ON coin_transactions
FOR EACH ROW EXECUTE FUNCTION sync_coin_balance();

-- ── Playtime ping log ──────────────────────────────────────────────────────
-- Tracks pings per user per game per day to enforce the daily cap.
CREATE TABLE IF NOT EXISTS playtime_pings (
  id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  game_id  UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  pinged_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_playtime_user_game_day
  ON playtime_pings(user_id, game_id, pinged_at);

-- ── Battle pass ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS battle_pass_seasons (
  id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name      TEXT NOT NULL,
  starts_at TIMESTAMPTZ NOT NULL,
  ends_at   TIMESTAMPTZ NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE IF NOT EXISTS battle_pass_tiers (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  season_id   UUID NOT NULL REFERENCES battle_pass_seasons(id) ON DELETE CASCADE,
  tier        INTEGER NOT NULL,
  xp_required INTEGER NOT NULL,
  reward_type TEXT NOT NULL, -- 'coins', 'badge', 'cosmetic', 'game_unlock'
  reward_meta JSONB NOT NULL,
  is_premium  BOOLEAN NOT NULL DEFAULT false,
  UNIQUE (season_id, tier)
);

CREATE TABLE IF NOT EXISTS user_battle_pass (
  user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  season_id    UUID NOT NULL REFERENCES battle_pass_seasons(id) ON DELETE CASCADE,
  is_premium   BOOLEAN NOT NULL DEFAULT false,
  current_tier INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id, season_id)
);

-- Track which tiers have been claimed to avoid double-claiming.
CREATE TABLE IF NOT EXISTS claimed_tiers (
  user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  season_id UUID NOT NULL REFERENCES battle_pass_seasons(id) ON DELETE CASCADE,
  tier      INTEGER NOT NULL,
  claimed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, season_id, tier)
);

-- ── Shop ───────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS shop_items (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name        TEXT NOT NULL,
  description TEXT,
  type        TEXT NOT NULL, -- 'badge', 'cosmetic', 'coin_boost', 'pass_upgrade'
  cost_coins  INTEGER NOT NULL,
  image_url   TEXT,
  is_active   BOOLEAN NOT NULL DEFAULT true,
  stock       INTEGER,       -- NULL = unlimited
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS user_purchases (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  item_id      UUID NOT NULL REFERENCES shop_items(id) ON DELETE RESTRICT,
  purchased_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── Mirror domains ─────────────────────────────────────────────────────────
-- Used by dailyspend.city/redirect when dawg.city is blocked.
CREATE TABLE IF NOT EXISTS mirrors (
  id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  url        TEXT NOT NULL,
  is_active  BOOLEAN NOT NULL DEFAULT true,
  added_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── Streak helper ──────────────────────────────────────────────────────────
-- Call this on every login to update streak and award streak coins/XP.
CREATE OR REPLACE FUNCTION update_streak(p_user_id UUID)
RETURNS INTEGER AS $$
DECLARE
  v_last_seen TIMESTAMPTZ;
  v_streak    INTEGER;
BEGIN
  SELECT last_seen, streak_days INTO v_last_seen, v_streak
  FROM users WHERE id = p_user_id;

  IF v_last_seen IS NULL OR v_last_seen < now() - INTERVAL '2 days' THEN
    UPDATE users SET streak_days = 1, last_seen = now() WHERE id = p_user_id;
    RETURN 1;
  ELSIF v_last_seen < now() - INTERVAL '1 day' THEN
    UPDATE users SET streak_days = streak_days + 1, last_seen = now() WHERE id = p_user_id;
    RETURN v_streak + 1;
  ELSE
    UPDATE users SET last_seen = now() WHERE id = p_user_id;
    RETURN v_streak;
  END IF;
END;
$$ LANGUAGE plpgsql;

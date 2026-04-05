# dawg.city — Full Build Plan
> Unblocked games portal with gamified loyalty system and coin economy across two domains.
> **Primary:** `dawg.city` — games portal | **Secondary:** `dailyspend.city` — coin shop + backup

---

## Stack Overview

| Layer | Technology | Notes |
|---|---|---|
| Frontend | Plain HTML + CSS + JS | Already your stack — keep it |
| Backend / API | Rust (Axum or Actix-web) | Proxy + REST API + OAuth |
| Database | PostgreSQL + PLpgSQL | Shared across both domains |
| CDN / DNS | Cloudflare | Both domains on same zone |
| Auth | Discord OAuth 2.0 | Low friction, no email required |
| Ad Network | PropellerAds or Monetag | Do NOT use AdSense initially |
| Analytics | PostHog (self-hosted or cloud) | A/B testing + funnels |
| Community | Discord server | Backup comms when domains blocked |

---

## Phase 0 — Archive & Reset ✅

**Goal:** Preserve the existing site, wipe public-facing content, deploy a holding page.

### 0.1 — Git archive ✅
> `archive.sh` created — run it to tag + push `v1-archive` and create the `v2-games` branch.

```bash
git tag v1-archive
git push origin v1-archive
git checkout -b v2-games
```

### 0.2 — Holding page ✅
> `static/holding.html` created — swap Discord invite URL, then replace `index.html` to go live.

Deploy a minimal holding page at `dawg.city` while you build. Keep it simple — a logo, a tagline, and a Discord invite link. This starts building your community before launch.

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>dawg.city — coming soon</title>
</head>
<body>
  <h1>dawg.city</h1>
  <p>Something big is coming. Join the Discord to be first.</p>
  <a href="https://discord.gg/YOUR_INVITE">Join Discord</a>
</body>
</html>
```

### 0.3 — Cloudflare setup

- Add both `dawg.city` and `dailyspend.city` to the same Cloudflare account
- Enable proxying (orange cloud) on all A records
- Set SSL mode to **Full (strict)**
- Enable **Bot Fight Mode** (protects against scrapers)
- Create a **Firewall Rule** to block known school-filter crawlers by user-agent if needed

### 0.4 — Register backup domains

Buy 2–3 additional domains now, before launch. Keep them parked and ready. When `dawg.city` gets blocked by a school filter, you point one of these at the same server and announce it in Discord.

Suggested naming pattern: `dawgcity.fun`, `playdawg.io`, `dawg.games`

---

## Phase 1 — Database Schema ✅

**Goal:** Design the full PostgreSQL schema that both domains share. Get this right before writing any application code.

> `supabase/schema.sql` created — run it in the Supabase SQL editor to apply all tables, indexes, triggers, and helper functions.

### 1.1 — Users table

```sql
CREATE TABLE users (
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
```

### 1.2 — Games table

```sql
CREATE TABLE games (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  slug         TEXT UNIQUE NOT NULL,       -- used in URL: /game/slope
  title        TEXT NOT NULL,
  description  TEXT,
  category     TEXT NOT NULL,              -- 'action', 'puzzle', 'io', 'racing', etc.
  embed_url    TEXT NOT NULL,              -- iframe src
  thumbnail    TEXT,
  tags         TEXT[],
  play_count   BIGINT NOT NULL DEFAULT 0,
  is_featured  BOOLEAN NOT NULL DEFAULT false,
  is_active    BOOLEAN NOT NULL DEFAULT true,
  added_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 1.3 — Leaderboards table

```sql
CREATE TABLE leaderboard (
  id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  game_id    UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  score      BIGINT NOT NULL,
  posted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (game_id, user_id)  -- one record per user per game, updated on improvement
);

CREATE INDEX idx_leaderboard_game_score ON leaderboard(game_id, score DESC);
```

### 1.4 — Coin transactions table

Every coin movement is logged — both earned and spent. Never mutate the `users.coins` column directly; always go through this table and use a trigger to keep it in sync.

```sql
CREATE TABLE coin_transactions (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  amount      INTEGER NOT NULL,            -- positive = earned, negative = spent
  reason      TEXT NOT NULL,              -- 'playtime', 'streak', 'purchase', 'battle_pass'
  meta        JSONB,                       -- optional context (game_id, item_id, etc.)
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Trigger to keep users.coins in sync
CREATE OR REPLACE FUNCTION sync_coin_balance()
RETURNS TRIGGER AS $$
BEGIN
  UPDATE users
  SET coins = coins + NEW.amount
  WHERE id = NEW.user_id;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER after_coin_transaction
AFTER INSERT ON coin_transactions
FOR EACH ROW EXECUTE FUNCTION sync_coin_balance();
```

### 1.5 — Battle pass table

```sql
CREATE TABLE battle_pass_seasons (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name        TEXT NOT NULL,               -- e.g. 'Season 1: Summer Grind'
  starts_at   TIMESTAMPTZ NOT NULL,
  ends_at     TIMESTAMPTZ NOT NULL,
  is_active   BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE battle_pass_tiers (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  season_id   UUID NOT NULL REFERENCES battle_pass_seasons(id),
  tier        INTEGER NOT NULL,            -- 1 through 30
  xp_required INTEGER NOT NULL,
  reward_type TEXT NOT NULL,              -- 'coins', 'badge', 'cosmetic', 'game_unlock'
  reward_meta JSONB NOT NULL,
  is_premium  BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE user_battle_pass (
  user_id     UUID NOT NULL REFERENCES users(id),
  season_id   UUID NOT NULL REFERENCES battle_pass_seasons(id),
  is_premium  BOOLEAN NOT NULL DEFAULT false,
  current_tier INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id, season_id)
);
```

### 1.6 — Shop items table (dailyspend.city)

```sql
CREATE TABLE shop_items (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name         TEXT NOT NULL,
  description  TEXT,
  type         TEXT NOT NULL,             -- 'badge', 'cosmetic', 'coin_boost', 'pass_upgrade'
  cost_coins   INTEGER NOT NULL,
  image_url    TEXT,
  is_active    BOOLEAN NOT NULL DEFAULT true,
  stock        INTEGER,                   -- NULL = unlimited
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_purchases (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id     UUID NOT NULL REFERENCES users(id),
  item_id     UUID NOT NULL REFERENCES shop_items(id),
  purchased_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 1.7 — Sessions table

```sql
CREATE TABLE sessions (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token       TEXT UNIQUE NOT NULL,
  expires_at  TIMESTAMPTZ NOT NULL,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sessions_token ON sessions(token);
```

---

## Phase 2 — Rust Backend (API) ✅

**Goal:** Build the Rust API that serves both domains. Use **Axum** for its async ergonomics and tower middleware compatibility.

> Implemented in `src/`. Set env vars `DATABASE_URL`, `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URI`, `FRONTEND_URL` before running.

### 2.1 — Project structure

```
dawg-api/
├── src/
│   ├── main.rs
│   ├── routes/
│   │   ├── auth.rs          # Discord OAuth
│   │   ├── games.rs         # game list, game page, play count
│   │   ├── leaderboard.rs   # read/write scores
│   │   ├── coins.rs         # playtime ping, coin balance
│   │   ├── shop.rs          # item list, purchase
│   │   └── battlepass.rs    # tier progress, claim reward
│   ├── middleware/
│   │   ├── auth.rs          # session token validation
│   │   └── cors.rs          # allow both domains
│   ├── models/              # DB row structs
│   ├── db.rs                # connection pool setup
│   └── proxy.rs             # game embed proxy (optional)
├── Cargo.toml
└── .env
```

### 2.2 — Key API endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/games` | List all games (with filter/search params) |
| `GET` | `/api/games/:slug` | Single game details |
| `POST` | `/api/games/:slug/ping` | Playtime heartbeat — awards coins (auth required) |
| `GET` | `/api/games/:slug/leaderboard` | Top scores for a game |
| `POST` | `/api/games/:slug/score` | Submit a score (auth required) |
| `GET` | `/api/auth/discord` | Redirect to Discord OAuth |
| `GET` | `/api/auth/callback` | Discord OAuth callback, issues session token |
| `GET` | `/api/me` | Authenticated user profile |
| `GET` | `/api/me/coins` | Coin balance + transaction history |
| `GET` | `/api/battlepass` | Current season + user's tier progress |
| `POST` | `/api/battlepass/claim/:tier` | Claim a tier reward (auth required) |
| `GET` | `/api/shop` | List all shop items |
| `POST` | `/api/shop/buy/:item_id` | Purchase an item with coins (auth required) |

### 2.3 — Playtime ping (coin award logic)

This is the core earning mechanic. The frontend sends a ping every 60 seconds while a game is open. The backend awards coins and XP per ping, capped to prevent abuse.

```rust
// POST /api/games/:slug/ping
// Requires: valid session token in Authorization header
// Awards: 5 coins + 10 Xp per ping, max 30 pings per day per game
async fn playtime_ping(
    State(db): State<DbPool>,
    session: Session,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let pings_today = count_pings_today(&db, session.user_id, &slug).await;
    if pings_today >= 30 {
        return (StatusCode::OK, Json(json!({ "awarded": false, "reason": "daily cap reached" })));
    }
    insert_coin_transaction(&db, session.user_id, 5, "playtime", Some(json!({ "game": slug }))).await;
    award_xp(&db, session.user_id, 10).await;
    check_battlepass_tier_up(&db, session.user_id).await;
    (StatusCode::OK, Json(json!({ "awarded": true, "coins": 5, "xp": 10 })))
}
```

### 2.4 — CORS configuration

Both domains must be allowed origins:

```rust
let cors = CorsLayer::new()
    .allow_origin([
        "https://dawg.city".parse::<HeaderValue>().unwrap(),
        "https://dailyspend.city".parse::<HeaderValue>().unwrap(),
    ])
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    .allow_credentials(true);
```

### 2.5 — Discord OAuth flow

```
1. User clicks "Login with Discord" on dawg.city
2. Browser redirects to GET /api/auth/discord
3. API redirects to Discord OAuth URL with client_id + scopes (identify)
4. User authorizes on Discord
5. Discord redirects to GET /api/auth/callback?code=...
6. API exchanges code for access token
7. API fetches user profile from Discord API
8. API upserts user in users table
9. API creates session token, sets HttpOnly cookie
10. API redirects back to dawg.city — user is now logged in
```

---

## Phase 3 — dawg.city Frontend ✅

**Goal:** The main games portal. Fast, clean, works on Chromebooks and school networks.

### 3.1 — Page structure

```
dawg.city/
├── index.html          # homepage — game grid, featured games, categories
├── game.html           # game page — full-screen iframe embed
├── leaderboard.html    # global + per-game leaderboards
├── profile.html        # user profile, coin balance, streak, progress
├── css/
│   ├── base.css
│   ├── grid.css
│   └── game.css
└── js/
    ├── auth.js          # session check, login/logout
    ├── coins.js         # display balance, ping loop
    └── games.js         # fetch + render game grid
```

### 3.2 — Homepage layout

```
┌──────────────────────────────────────────────────────────┐
│  DAWG.CITY      [Search...]     [Coins: 420]  [Profile]  │
├──────────────────────────────────────────────────────────┤
│  [Action] [Puzzle] [.io] [Racing] [Multiplayer] [New]    │
├──────────────────────────────────────────────────────────┤
│  ★ FEATURED ★                                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │ Slope    │  │ 1v1 LoL  │  │ Smash    │               │
│  └──────────┘  └──────────┘  └──────────┘               │
├──────────────────────────────────────────────────────────┤
│  ALL GAMES                                               │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐          │
│  │      │ │      │ │      │ │      │ │      │  ...      │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘          │
└──────────────────────────────────────────────────────────┘
```

### 3.3 — Game page layout

```
┌──────────────────────────────────────────────────────────┐
│  ← Back    SLOPE               [Coins: 420]  [Profile]   │
├─────────────────────────────────────┬────────────────────┤
│                                     │  Top scores        │
│                                     │  1. DawgKing  9999 │
│          [GAME IFRAME]              │  2. xX_pro_Xx 8821 │
│                                     │  3. you       7100 │
│                                     ├────────────────────┤
│                                     │  [AD UNIT]         │
└─────────────────────────────────────┴────────────────────┘
│  Related games                                           │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐                   │
└──────────────────────────────────────────────────────────┘
```

### 3.4 — Coin ping loop (JS)

```javascript
// js/coins.js
let pingInterval = null;

function startPingLoop(gameSlug) {
  const token = localStorage.getItem('session_token');
  if (!token) return; // not logged in, no coins earned

  pingInterval = setInterval(async () => {
    try {
      const res = await fetch(`https://api.dawg.city/api/games/${gameSlug}/ping`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      const data = await res.json();
      if (data.awarded) updateCoinDisplay(data.coins);
    } catch (e) {
      // silent fail — don't interrupt gameplay
    }
  }, 60_000); // every 60 seconds
}

function stopPingLoop() {
  if (pingInterval) clearInterval(pingInterval);
}

// Start when game loads, stop when user navigates away
document.addEventListener('DOMContentLoaded', () => {
  const slug = new URLSearchParams(location.search).get('game');
  if (slug) startPingLoop(slug);
});
window.addEventListener('beforeunload', stopPingLoop);
```

### 3.5 — Ad placement rules

- **Above the game iframe:** 728×90 leaderboard unit
- **Sidebar next to the game:** 300×250 rectangle unit
- **Between related games (homepage):** native/in-feed unit
- **NEVER:** inside the game iframe, covering the game, or as a pre-roll that blocks loading
- Use `display: none` on ad units on the game page if the user is logged in with a premium battle pass (optional premium perk)

---

## Phase 4 — dailyspend.city Frontend ✅

**Goal:** The coin shop and reward hub. Clean, browsable, works as a standalone site and as a backup portal.

> Implemented in `static/dailyspend/`. The server (`src/main.rs`) detects the `Host: dailyspend.city` header and rewrites paths to serve from `static/dailyspend/` via the `dailyspend_rewrite` middleware. `canonical_redirect` updated to handle both domains correctly (www → non-www, http → https).

### 4.1 — Page structure

```
dailyspend.city/
├── index.html          # shop homepage — item grid
├── item.html           # single item detail + buy button
├── profile.html        # coin balance, purchase history
├── redirect.html       # "dawg.city is blocked? click here" fallback portal
├── css/
│   └── shop.css
└── js/
    ├── auth.js          # same session token — shared with dawg.city
    └── shop.js          # fetch items, handle purchase
```

### 4.2 — Shop homepage layout

```
┌──────────────────────────────────────────────────────────┐
│  DAILYSPEND.CITY    Your balance: 🪙 420    [Profile]     │
├──────────────────────────────────────────────────────────┤
│  [All] [Badges] [Cosmetics] [Boosts] [Pass Upgrades]     │
├──────────────────────────────────────────────────────────┤
│  ┌────────────┐  ┌────────────┐  ┌────────────┐          │
│  │ 🐾 Badge   │  │ 🔥 Boost  │  │ ⭐ Premium  │         │
│  │ OG Dawg    │  │ 2x Coins  │  │ Pass       │         │
│  │ 🪙 250     │  │ 🪙 500    │  │ 🪙 1000    │         │
│  │ [Buy]      │  │ [Buy]     │  │ [Buy]      │         │
│  └────────────┘  └────────────┘  └────────────┘          │
└──────────────────────────────────────────────────────────┘
```

### 4.3 — Purchase flow

```javascript
// js/shop.js
async function buyItem(itemId) {
  const token = localStorage.getItem('session_token');
  if (!token) {
    alert('Log in on dawg.city first to earn and spend coins.');
    return;
  }

  const confirmed = confirm('Spend coins on this item?');
  if (!confirmed) return;

  const res = await fetch(`https://api.dawg.city/api/shop/buy/${itemId}`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}` }
  });

  const data = await res.json();
  if (data.success) {
    document.getElementById('balance').textContent = data.new_balance;
    showToast('Purchase successful! 🐾');
  } else {
    showToast(`Could not purchase: ${data.error}`);
  }
}
```

### 4.4 — Backup redirect page

When `dawg.city` is blocked, users can visit `dailyspend.city/redirect` to find the current working game URL.

```html
<!-- redirect.html -->
<h1>dawg.city blocked?</h1>
<p>Use one of these links instead:</p>
<ul id="mirrors">
  <!-- populated from API: GET /api/mirrors -->
</ul>
<script>
  fetch('https://api.dawg.city/api/mirrors')
    .then(r => r.json())
    .then(links => {
      document.getElementById('mirrors').innerHTML =
        links.map(l => `<li><a href="${l.url}">${l.url}</a></li>`).join('');
    });
</script>
```

Store the current working mirror URLs in a simple DB table and update it whenever you rotate domains.

---

## Phase 5 — Gamification Engine

**Goal:** The feature that differentiates dawg.city from every generic game site. Build this after core functionality works.

### 5.1 — Coin earn rates

| Action | Coins | XP | Cap |
|---|---|---|---|
| Playing a game (per 60s ping) | +5 | +10 | 30 pings/day/game |
| Daily login streak (day 1–6) | +10 | +20 | Once per day |
| Daily login streak (day 7+) | +50 | +100 | Once per week |
| Posting a leaderboard score | +15 | +30 | Once per game per day |
| Beating your personal best | +25 | +50 | Once per game per day |

### 5.2 — Battle pass structure

- **Duration:** 90 days per season
- **Tiers:** 30 tiers total
- **Free track:** coins, small badges at tiers 5, 10, 20, 30
- **Premium track:** cosmetics, coin boosts, exclusive game unlocks, animated badges
- **XP to tier up:** 500 XP per tier (15,000 XP to complete the pass)
- **Premium upgrade cost:** 1,000 coins (purchasable on dailyspend.city)

### 5.3 — Leaderboard rules

- One score per user per game — updated only if the new score is higher
- Global top 100 displayed per game
- Personal rank always shown even if outside top 100
- Weekly reset option (configurable per game via `leaderboard_reset` column on games table)
- Seasonal all-time leaderboard frozen at season end and archived

### 5.4 — Streak system

```sql
-- PLpgSQL function: call on every login
CREATE OR REPLACE FUNCTION update_streak(p_user_id UUID)
RETURNS INTEGER AS $$
DECLARE
  v_last_seen TIMESTAMPTZ;
  v_streak    INTEGER;
BEGIN
  SELECT last_seen, streak_days INTO v_last_seen, v_streak
  FROM users WHERE id = p_user_id;

  IF v_last_seen IS NULL OR v_last_seen < now() - INTERVAL '2 days' THEN
    -- Streak broken or first login
    UPDATE users SET streak_days = 1, last_seen = now() WHERE id = p_user_id;
    RETURN 1;
  ELSIF v_last_seen < now() - INTERVAL '1 day' THEN
    -- Consecutive day
    UPDATE users SET streak_days = streak_days + 1, last_seen = now() WHERE id = p_user_id;
    RETURN v_streak + 1;
  ELSE
    -- Already logged in today
    UPDATE users SET last_seen = now() WHERE id = p_user_id;
    RETURN v_streak;
  END IF;
END;
$$ LANGUAGE plpgsql;
```

---

## Phase 6 — Monetization

**Goal:** Set up ad revenue before launch. Ads are the primary income source.

### 6.1 — Ad network setup

1. Apply to **PropellerAds** or **Monetag** first — both accept unblocked games sites
2. Do NOT apply to Google AdSense until you have 3+ months of traffic history and a clean site category
3. Once approved, you'll receive JavaScript ad tags to embed in pages
4. Place tags in your HTML templates — not in game iframes

### 6.2 — Ad placement priority

| Placement | Unit size | Expected CPM |
|---|---|---|
| Game page sidebar | 300×250 | Highest |
| Above game iframe | 728×90 | High |
| Homepage between rows | Native/in-feed | Medium |
| Profile / leaderboard pages | 300×250 | Medium |

### 6.3 — A/B testing with PostHog

Install PostHog (free tier works) and test:
- Related games widget placement (above vs below iframe)
- Number of games shown on homepage (12 vs 24 vs 48)
- Login prompt timing (immediate vs after 2 minutes of play)
- Coin earn rate messaging ("You earned 5 coins!" vs silent)

Track: pageviews per session, time on site, return visit rate, login conversion rate.

---

## Phase 7 — Growth & Resilience

**Goal:** Build a user base and a system that survives domain blocks.

### 7.1 — Pre-launch checklist

- [ ] Discord server live with at least a #announcements and #new-domain channels
- [ ] 50+ games embedded and tested on Chromebook + Chrome with extensions disabled
- [ ] All pages load under 2 seconds on a throttled connection
- [ ] Auth flow working end-to-end
- [ ] At least one battle pass season configured
- [ ] Ad tags live and serving
- [ ] At least 1 backup domain registered and pointed at same server

### 7.2 — Content calendar

Weekly drops keep the site feeling alive and give users a reason to return.

| Day | Action |
|---|---|
| Monday | Drop 3–5 new games, post in Discord |
| Wednesday | Announce leaderboard winners from prior week |
| Friday | Post weekend challenge (specific game, prize coins) |
| End of season | Freeze leaderboards, announce new season theme |

### 7.3 — Domain rotation protocol

When `dawg.city` gets blocked by a school network:

1. Announce new mirror URL in Discord #new-domain immediately
2. Update `dailyspend.city/redirect` page with the new URL (update DB mirrors table)
3. Point backup domain to same server via Cloudflare in under 5 minutes
4. Post on TikTok / any social with new URL if the block is widespread

### 7.4 — TikTok / Shorts funnel

Short-form video is the highest-ROI growth channel for this audience.

- Record 30–60 second gameplay clips of popular games
- Caption: "play this at school → dawg.city"
- Post 3–5x per week consistently
- Pin a comment with the current working URL
- When a video pops off, update the pinned comment if the domain has rotated

---

## Summary: Build Order

```
Phase 0  →  Archive + holding page + Cloudflare setup     (1 day)
Phase 1  →  Full DB schema + migrations                    (2–3 days)
Phase 2  →  Rust API: auth + games + coins endpoints       (1–2 weeks)
Phase 3  →  dawg.city frontend: homepage + game page       (1 week)
Phase 4  →  dailyspend.city frontend: shop + redirect      (3–5 days)
Phase 5  →  Gamification: battle pass + streaks + LB       (1 week)
Phase 6  →  Ad network signup + placement                  (1–2 days)
Phase 7  →  Launch + Discord + content calendar            (ongoing)
```

**Total to MVP:** approximately 5–7 weeks of focused part-time work.

---

*dawg.city build plan — generated for internal use*

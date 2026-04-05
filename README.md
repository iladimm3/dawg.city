# dawg.city

Free unblocked games portal with a gamified coin economy — built for school networks.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## What it is

**dawg.city** — Play hundreds of free games, earn coins just by playing, climb leaderboards, and unlock rewards through a battle pass.

**dailyspend.city** — The companion coin shop. Spend earned coins on badges, boosts, and pass upgrades.

Both sites run on a single Rust server with a shared PostgreSQL database. User accounts are Discord OAuth — no email required.

---

## Stack

| Layer | Technology |
|---|---|
| Frontend | Plain HTML + CSS + JS (no framework) |
| Backend | Rust (Axum) |
| Database | PostgreSQL (Supabase) |
| Auth | Discord OAuth 2.0 |
| CDN / DNS | Cloudflare |
| Deploy | Railway (Docker) |

---

## Project structure

```
src/
  main.rs             HTTP server — routing, CORS, host-based rewriting
  db.rs               PostgreSQL connection pool
  routes/
    auth.rs           Discord OAuth + session management
    games.rs          Game list, play count, playtime ping (coin award)
    leaderboard.rs    Score submission + ranked reads
    coins.rs          Coin balance + transaction history
    battlepass.rs     Season progress + tier claim
    shop.rs           Shop item list + purchase
  middleware/
    auth.rs           Session token validation (Bearer header)
  models/
    mod.rs            DB row structs (User, Game, ShopItem, …)

static/
  index.html          dawg.city homepage — game grid + search
  game.html           Full-screen game embed + leaderboard sidebar
  leaderboard.html    Global + per-game leaderboards
  profile.html        Coin balance, XP, streak, battle pass progress
  css/
    base.css          Design tokens, topbar, buttons, toasts
    grid.css          Game card grid + skeleton loaders
    game.css          Game embed layout + sidebar
  js/
    auth.js           Session token management + auth area render
    coins.js          Coin display + 60s playtime ping loop
    games.js          Fetch + render game grid

  dailyspend/         Served at dailyspend.city (host-rewrite middleware)
    index.html        Coin shop homepage — filterable item grid
    item.html         Item detail page + buy button
    profile.html      Coin balance + purchase history
    redirect.html     Mirror finder — "dawg.city blocked? try these"
    css/shop.css      All styles for dailyspend.city
    js/auth.js        Session management (shared token with dawg.city)
    js/shop.js        Fetch items, filter by type, purchase flow

supabase/
  schema.sql          Full DB schema — users, games, leaderboard, coins,
                      battle pass, shop, sessions, mirrors
  profiles.sql        Supabase profile helpers (if needed)
  jobs.sql            Background job queue schema
```

---

## Environment variables

| Variable | Description |
|---|---|
| `DATABASE_URL` | PostgreSQL connection string |
| `DISCORD_CLIENT_ID` | Discord OAuth app client ID |
| `DISCORD_CLIENT_SECRET` | Discord OAuth app client secret |
| `DISCORD_REDIRECT_URI` | OAuth callback URL (e.g. `https://dawg.city/api/auth/callback`) |
| `FRONTEND_URL` | Base URL for post-auth redirect (e.g. `https://dawg.city`) |
| `PORT` | HTTP port — injected automatically by Railway |

---

## Running locally

```bash
# 1. Copy and fill environment variables
cp .env.example .env

# 2. Apply the database schema
#    Run supabase/schema.sql in your Supabase SQL editor

# 3. Start the server
PORT=3000 ~/.cargo/bin/cargo run --bin server

# 4. Open http://localhost:3000
```

---

## Deploying to Railway

### Server service

1. Create a Railway project and link this repo.
2. Railway auto-detects `Dockerfile` — builds the `server` binary.
3. Set all environment variables listed above. `PORT` is injected automatically.
4. Health check is configured at `GET /health` via `railway.toml`.
5. Both `dawg.city` and `dailyspend.city` point to the same Railway service via Cloudflare.

### Domain routing

The server detects the `Host` header:

- `Host: dawg.city` → serves `static/` (games portal)
- `Host: dailyspend.city` → path-rewrites to `static/dailyspend/` (coin shop)
- All `/api/*` routes are shared across both domains

### Cloudflare setup

- Both domains on the same Cloudflare account.
- SSL mode: **Full (strict)**.
- Both A records point to the Railway service IP with orange-cloud proxying enabled.
- `www.*` → canonical redirect handled by the server middleware.

---

## API endpoints

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/auth/discord` | — | Redirect to Discord OAuth |
| `GET` | `/api/auth/callback` | — | OAuth callback, issues session |
| `GET` | `/api/me` | ✓ | Authenticated user profile |
| `GET` | `/api/me/coins` | ✓ | Coin balance + history + purchases |
| `GET` | `/api/mirrors` | — | Active mirror URLs for redirect page |
| `GET` | `/api/games` | — | List games (`?category=&search=&featured=true`) |
| `GET` | `/api/games/:slug` | — | Single game details |
| `POST` | `/api/games/:slug/ping` | ✓ | Playtime heartbeat — awards 5 coins + 10 XP (cap: 30/day/game) |
| `GET` | `/api/games/:slug/leaderboard` | — | Top scores for a game |
| `POST` | `/api/games/:slug/score` | ✓ | Submit score (kept if higher than personal best) |
| `GET` | `/api/battlepass` | ✓ | Current season + tier progress |
| `POST` | `/api/battlepass/claim/:tier` | ✓ | Claim a tier reward |
| `GET` | `/api/shop` | — | List active shop items |
| `POST` | `/api/shop/buy/:item_id` | ✓ | Purchase an item with coins |
| `GET` | `/health` | — | Healthcheck |

---

## Coin earn rates

| Action | Coins | XP | Cap |
|---|---|---|---|
| Playing (60s ping) | +5 | +10 | 30 pings/day/game |
| Daily streak day 1–6 | +10 | +20 | Once/day |
| Daily streak day 7+ | +50 | +100 | Once/week |
| Posting a score | +15 | +30 | Once/game/day |
| Beating personal best | +25 | +50 | Once/game/day |

---

## Build status

Phases completed: **0 → 4** (archive → DB schema → Rust API → dawg.city frontend → dailyspend.city frontend)

Next: Phase 5 — Gamification Engine (battle pass UI, streak system, leaderboard reset jobs)

---

## License

MIT


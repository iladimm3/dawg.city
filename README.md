# 🐾 Dawg City API

AI-powered dog training & nutrition backend built with Axum (Rust) + PostgreSQL.

## Stack
- **Framework**: Axum 0.7
- **Database**: PostgreSQL via SQLx
- **Auth**: Google OAuth2 (signed cookie sessions)
- **AI**: Anthropic Claude (training + nutrition plans)
- **Hosting**: Railway

## Local Setup

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install sqlx-cli for migrations
cargo install sqlx-cli --no-default-features --features postgres

# 3. Copy env file and fill in values
cp .env.example .env

# 4. Create local DB and run migrations
createdb dawgcity
sqlx migrate run

# 5. Run
cargo run
```

## API Routes

### Auth
| Method | Path | Description |
|--------|------|-------------|
| GET | /auth/google | Redirect to Google login |
| GET | /auth/google/callback | OAuth callback |
| GET | /auth/logout | Clear session |
| GET | /auth/me | Current user info |

### Dogs (🔒 requires auth)
| Method | Path | Description |
|--------|------|-------------|
| GET | /api/dogs | List your dogs |
| POST | /api/dogs | Create dog profile |
| GET | /api/dogs/:id | Get single dog |
| PUT | /api/dogs/:id | Update dog |
| DELETE | /api/dogs/:id | Delete dog |

### Training (🔒 requires auth)
| Method | Path | Description |
|--------|------|-------------|
| POST | /api/training/session | Generate AI training session |
| POST | /api/training/log | Log session result |
| GET | /api/training/history?dog_id=&limit=&offset= | Paginated training history |

### Nutrition (🔒 requires auth)
| Method | Path | Description |
|--------|------|-------------|
| POST | /api/nutrition/plan | Generate AI nutrition plan |

### System
| Method | Path | Description |
|--------|------|-------------|
| GET | /health | Health check + DB status |

## Deploy to Railway

```bash
# Install Railway CLI
npm install -g @railway/cli

# Login and deploy
railway login
railway init
railway add --plugin postgresql
railway up
```

Set env vars in Railway dashboard matching `.env.example`.

## Environment Variables

| Variable | Required | Example |
|----------|----------|---------|
| `DATABASE_URL` | ✅ | `postgres://user:pass@host/db` |
| `GOOGLE_CLIENT_ID` | ✅ | `123456.apps.googleusercontent.com` |
| `GOOGLE_CLIENT_SECRET` | ✅ | `GOCSPX-...` |
| `GOOGLE_REDIRECT_URI` | ✅ | `https://your-domain/auth/google/callback` |
| `COOKIE_SECRET` | ✅ | 64-char random hex string |
| `ANTHROPIC_API_KEY` | ✅ | `sk-ant-...` |
| `ANTHROPIC_MODEL` | optional | `claude-sonnet-4-20250514` |
| `PORT` | optional | `3000` |
| `RUST_LOG` | optional | `info` |

## Google OAuth Setup
1. Go to [console.cloud.google.com](https://console.cloud.google.com)
2. Create project → APIs & Services → Credentials
3. Create OAuth 2.0 Client ID (Web application)
4. Add redirect URI: `https://your-railway-domain/auth/google/callback`
5. Copy Client ID + Secret to Railway env vars

## Development Roadmap

See [ROADMAP.md](ROADMAP.md) for the full development roadmap (phases 1-5, completed items, testing tasks, and technical debt).

# dawg.city — AI video deepfake detection

[![CI](https://github.com/iladimm3/dawg.city/actions/workflows/worker-ci.yml/badge.svg?branch=main)](https://github.com/iladimm3/dawg.city/actions/workflows/worker-ci.yml)
[![Secret Scan](https://github.com/iladimm3/dawg.city/actions/workflows/secret-scan.yml/badge.svg?branch=main)](https://github.com/iladimm3/dawg.city/actions/workflows/secret-scan.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Lightweight production platform to detect AI-generated video frames using a static frontend and a Rust HTTP server deployed on Railway.

Overview
--------
`dawg.city` provides fast, opinionated detection for social video platforms (YouTube, TikTok, X/Twitter, Instagram) using a static site (Tailwind + Vanilla JS) and an Axum-based Rust server deployed on Railway.

Badges
------
- **CI**: status for core Rust checks and tests (see workflow `worker-ci.yml`)
- **Secret Scan**: automated repository secret scanning (truffleHog + detect-secrets)
- **License**: MIT

Key features
- Minimal, fast static frontend
- Axum HTTP server with `POST /api/analyze` and `POST /api/webhook` endpoints
- Static file serving via `tower-http::ServeDir`
- Asynchronous processing via job queue + worker (Supabase-backed)
- Sightengine AI detection for thumbnail/frame scoring
- Supabase for auth and quota tracking, Stripe for billing
- Dockerized deployment on Railway

Quick start
-----------
1. Fork or clone the repository.
2. Install Rust (1.77+).
3. Set required environment variables (see below).
4. Run locally:
   ```bash
   PORT=3000 cargo run --bin server
   ```
5. Open `http://localhost:3000/`.

Deploy to Railway
-----------------
**Server service:**
1. Create a new Railway project and link this repo.
2. Railway will auto-detect the `Dockerfile` and build the `server` binary.
3. Set all required environment variables (see below). `PORT` is injected automatically.
4. The healthcheck is configured at `GET /health` via `railway.toml`.

**Worker service:**
1. Add a second service in the same Railway project.
2. Point it to `Dockerfile.worker`.
3. Set `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY`, `SIGHTENGINE_API_USER`, `SIGHTENGINE_API_SECRET`.
4. No port needed — the worker is a long-lived polling process.

**Docker (manual):**
```bash
docker build -t dawg-city .
docker run -p 3000:3000 -e PORT=3000 \
  -e RECAPTCHA_SECRET=... \
  -e SUPABASE_URL=... \
  -e SUPABASE_SERVICE_ROLE_KEY=... \
  -e STRIPE_WEBHOOK_SECRET=... \
  -e SIGHTENGINE_API_USER=... \
  -e SIGHTENGINE_API_SECRET=... \
  dawg-city
```

Captures d'écran
----------------
Voici quelques captures d'écran (remplacez par vos propres images si vous préférez) :

![Dashboard placeholder](https://placehold.co/800x450?text=Dashboard)

![Scan result placeholder](https://placehold.co/800x450?text=Scan+Result)

Environment variables
---------------------
Set these in Railway's service variables or in your local environment.

| Variable | Required | Used By | Purpose |
|---|---|---|---|
| `PORT` | auto | server | Railway injects automatically |
| `ALLOWED_ORIGIN` | optional | server | CORS origin (default: `https://dawg.city`) |
| `RECAPTCHA_SECRET` | yes | server | Google reCAPTCHA v3 validation |
| `SUPABASE_URL` | yes | server, worker | Supabase API endpoint |
| `SUPABASE_SERVICE_ROLE_KEY` | yes | server, worker | Server auth for Supabase |
| `STRIPE_WEBHOOK_SECRET` | yes | server | Stripe signature verification |
| `SIGHTENGINE_API_USER` | yes | server | AI detection service |
| `SIGHTENGINE_API_SECRET` | yes | server | AI detection service |
| `INSTAGRAM_TOKEN` | optional | server | Facebook Graph oEmbed |

Run the worker locally
----------------------
The worker binary polls a `jobs` table in Supabase, claims and processes jobs, and writes results back.

```bash
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_ROLE_KEY="your-service-role-key"
export SIGHTENGINE_API_USER="..."
export SIGHTENGINE_API_SECRET="..."

cargo run --bin worker
```

Project structure
-----------------
```
src/
  main.rs        # Axum server entrypoint
  analyze.rs     # POST /api/analyze handler
  webhook.rs     # POST /api/webhook handler (Stripe)
  worker.rs      # Async job processor (separate binary)
static/          # Served by Axum fallback (ServeDir)
  index.html, blog.html, about.html, ...
  style.css, script.js
  posts/
Dockerfile       # Multi-stage build for server
Dockerfile.worker
railway.toml     # Railway deployment config
```

Development & CI
-----------------
- Unit tests and network-mocked tests are included for worker helpers.
- A GitHub Actions workflow runs `cargo fmt`, `cargo clippy`, `cargo build`, `cargo test`, and `cargo audit` on `main`.

Architecture notes
------------------
For a full architecture assessment, see the repository's architecture review: [ARCHITECTURE_REVIEW.md](ARCHITECTURE_REVIEW.md)

Contributing
------------
Feel free to open issues or PRs. Key areas where contributions help:
- CI and tests for production handlers
- Robust HF integration and error handling
- Quota enforcement and edge rate-limiting

License
-------
MIT

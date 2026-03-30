# Railway Migration Roadmap

## TL;DR

Replace Vercel serverless Rust functions with a single **Axum** HTTP server that serves static files and API routes, containerized via Dockerfile, deployed on **Railway**.

The Vercel-specific `vercel_runtime` crate is deeply embedded in `analyze.rs` and `webhook.rs` — both handlers use `vercel_runtime::{Request, Response, Body, StatusCode}`. These must be rewritten to use Axum (lightweight, tokio-native) as the HTTP framework. The `worker.rs` binary is already standalone (no Vercel dependency) and can be deployed as a separate Railway service.

---

## Phase 1: Replace Vercel Runtime with Axum

### 1. Update `Cargo.toml`

- Remove `vercel_runtime` dependency
- Add `axum = "0.7"`, `tower-http = { version = "0.5", features = ["cors", "fs"] }` for CORS middleware and static file serving
- Change `tokio` features from `["macros"]` to `["full"]`
- Replace the 3 `[[bin]]` targets with:
  - `[[bin]] name = "server" path = "src/main.rs"` — main web server
  - `[[bin]] name = "worker" path = "src/worker.rs"` — async job processor

### 2. Create `src/main.rs` — Axum server entrypoint

- Build an Axum `Router` with:
  - `POST /api/analyze` → analyze handler
  - `POST /api/webhook` → webhook handler
  - `GET /health` → healthcheck endpoint (returns 200)
  - Fallback → `tower_http::services::ServeDir` serving static files from `./static/`
- CORS layer via `tower_http::cors::CorsLayer`
  - Allow origin: `https://dawg.city` (make configurable via `ALLOWED_ORIGIN` env var)
  - Allow methods: `POST, OPTIONS`
  - Allow headers: `Content-Type, Authorization`
- Bind to `0.0.0.0:$PORT` (Railway injects `PORT` env var)
- Graceful shutdown with `tokio::signal`

### 3. Refactor `api/analyze.rs` → `src/analyze.rs`

- Replace `use vercel_runtime::{...}` with `use axum::{...}`
- Change handler signature:
  - **Before:** `pub async fn handler(req: Request) -> Result<Response<Body>, Error>`
  - **After:** `pub async fn handler(headers: HeaderMap, body: String) -> impl IntoResponse`
- Replace `Body::Text(s)` / `Body::Binary(b)` with standard Axum body handling
- Replace `Response::builder().status(...).header(...).body(Body::Text(...))` with Axum tuple responses `(StatusCode, [(header, value)], Json(...))`
- Replace `x-vercel-id` request ID with a generated UUID per-request
- Remove `run(handler).await` from `main()`
- **Keep all business logic unchanged**: reCAPTCHA, Supabase, Sightengine, platform detection

### 4. Refactor `api/webhook.rs` → `src/webhook.rs`

- Same migration pattern as analyze.rs
- Change handler to accept `axum::body::Bytes` (raw body needed for HMAC verification) + `HeaderMap`
- **Keep Stripe signature verification logic intact**
- Keep all Supabase upsert logic unchanged

### 5. Move `api/worker.rs` → `src/worker.rs`

- No code changes needed — it doesn't use `vercel_runtime`
- Only update the path in `Cargo.toml`

---

## Phase 2: Static File Serving & Project Structure

### 6. Move static files to `static/` directory

Move the following files into `static/`:

| File(s) | Purpose |
|---|---|
| `index.html` | Main landing/detector page |
| `blog.html` | Blog page |
| `about.html` | About page |
| `upgrade.html` | Pricing/upgrade page |
| `privacy.html` | Privacy policy |
| `cookies.html` | Cookie policy |
| `style.css` | Tailwind CSS compiled |
| `script.js` | Frontend logic |
| `robots.txt` | SEO |
| `sitemap.xml` | SEO |
| `ads.txt` | AdSense verification |
| `posts/` | Blog post pages (entire directory) |

These will be served by Axum's `ServeDir` as the fallback route.

---

## Phase 3: Dockerfile & Railway Config

### 7. Create `Dockerfile` (multi-stage build)

```dockerfile
# Stage 1: Build
FROM rust:1.77-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --bin server

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/server .
COPY static/ static/
EXPOSE 8080
CMD ["./server"]
```

### 8. Create `railway.toml`

- Configure build from Dockerfile
- Set healthcheck path: `GET /health`
- Reference required environment variables

### 9. Second Railway service for `worker`

- Separate Dockerfile target (or `Dockerfile.worker`) for the worker binary
- Runs as a long-lived process (not a web service — no port needed)
- Same env vars: `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY`, `SIGHTENGINE_*`

---

## Phase 4: Cleanup

### 10. Remove `vercel.json`

### 11. Update `README.md` with Railway deployment instructions

---

## File Map

| Current File | Action | New Location |
|---|---|---|
| `Cargo.toml` | Edit | `Cargo.toml` |
| `api/analyze.rs` | Rewrite (Vercel → Axum) | `src/analyze.rs` |
| `api/webhook.rs` | Rewrite (Vercel → Axum) | `src/webhook.rs` |
| `api/worker.rs` | Move (no code changes) | `src/worker.rs` |
| `vercel.json` | Delete | — |
| — | Create | `src/main.rs` |
| — | Create | `Dockerfile` |
| — | Create | `Dockerfile.worker` |
| — | Create | `railway.toml` |
| `*.html`, `style.css`, `script.js`, etc. | Move | `static/` |

---

## Environment Variables (Required on Railway)

| Variable | Required | Used By | Purpose |
|---|---|---|---|
| `PORT` | ✅ (auto) | server | Railway injects this automatically |
| `ALLOWED_ORIGIN` | ⚠️ | server | CORS origin (default: `https://dawg.city`) |
| `RECAPTCHA_SECRET` | ✅ | analyze | Google reCAPTCHA v3 validation |
| `SUPABASE_URL` | ✅ | analyze, webhook, worker | Supabase API endpoint |
| `SUPABASE_SERVICE_ROLE_KEY` | ✅ | analyze, webhook, worker | Server auth for Supabase |
| `STRIPE_WEBHOOK_SECRET` | ✅ | webhook | Stripe signature verification |
| `SIGHTENGINE_API_USER` | ✅ | analyze | AI detection service |
| `SIGHTENGINE_API_SECRET` | ✅ | analyze | AI detection service |
| `INSTAGRAM_TOKEN` | ⚠️ | analyze, worker | Facebook Graph oEmbed (optional) |
| `HF_API_KEY` | ⚠️ | worker | Hugging Face inference (optional) |
| `HF_MODEL` | ⚠️ | worker | HF model ID (default: `naman712/seedance`) |

---

## Verification Checklist

- [ ] `cargo build --release --bin server` compiles without errors
- [ ] `cargo build --release --bin worker` still compiles
- [ ] `cargo test` — all existing tests pass (with adjusted imports)
- [ ] Local test: `PORT=3000 cargo run --bin server`
  - [ ] `curl http://localhost:3000/` returns index.html
  - [ ] `curl -X POST http://localhost:3000/api/analyze` returns expected error
  - [ ] `curl -X OPTIONS http://localhost:3000/api/analyze` returns CORS headers
  - [ ] `curl http://localhost:3000/health` returns 200
- [ ] `docker build -t dawg-city .` succeeds
- [ ] `docker run -p 3000:3000 -e PORT=3000 dawg-city` serves correctly
- [ ] Deploy to Railway, verify all env vars are set, test live endpoints
- [ ] Stripe webhooks point to new Railway URL
- [ ] DNS/domain `dawg.city` points to Railway

---

## Architecture Decisions

| Decision | Rationale |
|---|---|
| **Axum** over Actix-web | Tokio-native (already using tokio), lighter weight, tower ecosystem for middleware (CORS, static files) |
| **Single server binary** | Merging analyze + webhook into one server is simpler for Railway (1 service = 1 port) |
| **Worker as separate service** | Long-lived polling process — should scale/restart independently |
| **Static files via Axum** | No need for nginx; Railway can handle it. CDN (Cloudflare) can be added later |

---

## Notes & Gotchas

1. **CORS origin**: Currently hardcoded to `https://dawg.city`. During initial Railway testing, the app will be at `*.up.railway.app` — use the `ALLOWED_ORIGIN` env var to configure this.
2. **`x-vercel-id` replacement**: The analyze handler logs `x-vercel-id` as `request_id`. Replace with a `uuid::Uuid::new_v4()` generated per-request.
3. **Stripe webhook URL**: After deployment, update the webhook endpoint URL in the Stripe Dashboard to point to the new Railway domain.
4. **`ca-certificates`**: The Dockerfile runtime image must include `ca-certificates` for TLS connections to Supabase, Sightengine, Google, etc.
5. **No timeout constraint**: Vercel had a 10s function timeout. Railway has no such limit, which benefits long-running analysis requests.

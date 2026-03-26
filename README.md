# dawg.city — AI video deepfake detection

[![CI](https://github.com/iladimm3/dawg.city/actions/workflows/worker-ci.yml/badge.svg?branch=main)](https://github.com/iladimm3/dawg.city/actions/workflows/worker-ci.yml)
[![Secret Scan](https://github.com/iladimm3/dawg.city/actions/workflows/secret-scan.yml/badge.svg?branch=main)](https://github.com/iladimm3/dawg.city/actions/workflows/secret-scan.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Lightweight production platform to detect AI-generated video frames using a static frontend and small Rust serverless APIs.

Overview
--------
`dawg.city` provides fast, opinionated detection for social video platforms (YouTube, TikTok, X/Twitter, Instagram) using a static site (Tailwind + Vanilla JS) and Rust serverless functions deployed on Vercel.

Badges
------
- **CI**: status for core Rust checks and tests (see workflow `worker-ci.yml`)
- **Secret Scan**: automated repository secret scanning (truffleHog + detect-secrets)
- **License**: MIT

Key features
- Minimal, fast static frontend
- Rust serverless endpoints: `analyze` and `webhook`
- Asynchronous processing via job queue + worker (Supabase-backed)
- Hugging Face inference for thumbnail/frame scoring
- Supabase for auth and quota tracking, Stripe for billing

Quick start
-----------
1. Fork or clone the repository.
2. Add required environment variables in Vercel or locally (see below).
3. Deploy to Vercel (auto-detects Rust runtime) or run locally for development.

Captures d'écran
----------------
Voici quelques captures d'écran (remplacez par vos propres images si vous préférez) :

![Dashboard placeholder](https://placehold.co/800x450?text=Dashboard)

![Scan result placeholder](https://placehold.co/800x450?text=Scan+Result)

Environment variables (examples)
--------------------------------
Set these in Vercel's dashboard or in your local environment when running locally.

```
# AI / inference
HF_API_KEY=your_huggingface_api_key
HF_MODEL=naman712/seedance

# Sightengine (optional)
SIGHTENGINE_API_USER=your_sightengine_user
SIGHTENGINE_API_SECRET=your_sightengine_secret

# Supabase
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key

# Payments
STRIPE_SECRET_KEY=your_stripe_secret_key
STRIPE_WEBHOOK_SECRET=your_stripe_webhook_signing_secret

# Optional
INSTAGRAM_TOKEN=your_instagram_graph_token
RECAPTCHA_SECRET=your_recaptcha_secret_key
```

Run the worker locally
----------------------
The project includes a prototype worker binary that polls a `jobs` table in Supabase, claims and processes jobs, and writes results back.

```bash
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_ROLE_KEY="your-service-role-key"
export HF_API_KEY="hf_xxx"              # optional but required for real HF inference
export HF_MODEL="naman712/seedance"

cargo run --bin worker
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

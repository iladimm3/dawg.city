# 🐾 Dawg City — Development Roadmap

## Phase 1 — Data Persistence & History
**Priority: High** — closes the biggest functional gaps

| Item | Status | Notes |
|------|--------|-------|
| Save nutrition plans to DB (`nutrition_plans` table) | ✅ Done | Auto-saved on every generation via `POST /api/nutrition/plan` |
| Auto-save training sessions on generation | ✅ Done | Creates a `completed: false` log with `log_id` returned to frontend |
| Nutrition history page (review past plans) | ✅ Done | History tab in Nutrition page, `GET /api/nutrition/history` |
| Robust Anthropic API error handling | ✅ Done | Shared `services::anthropic` module with timeout, status checks, explicit errors |
| Cache invalidation after nutrition plan generation | ✅ Done | `queryClient.invalidateQueries` in mutation `onSuccess` |

---

## Phase 2 — Subscription & Monetisation
The `subscription_tier` column already exists in the `users` table but nothing uses it yet.

- [ ] Stripe integration (Checkout session creation, webhook handling)
- [ ] Tier-based gating: limit AI generations for `free` tier, unlock for `pro`
- [ ] Subscription management UI (upgrade, billing portal, cancellation)
- [ ] Webhook: sync Stripe events → `subscription_tier` in DB
- [ ] **Testing:** Stripe webhook signature verification test; generation limit enforcement test

---

## Phase 3 — Enriched Dog Profiles
- [ ] Dog photo upload (S3/CDN storage) — frontend currently shows a `PawPrint` placeholder everywhere
- [ ] Multi-step onboarding (aligned with DESIGN.md "Joyful Guardian" vision)
- [ ] Breed auto-suggest + enriched breed data (typical weight, lifespan, common health issues)
- [ ] `photo_url` column already added to `dogs` via migration 0002 — just needs wiring up
- [ ] **Testing:** File upload size/type validation test; breed data integrity test

---

## Phase 4 — Analytics & Progression
- [ ] Training progression charts (weekly/monthly trends using `training_logs` data)
- [ ] Nutrition tracking over time (calorie history, weight trend)
- [ ] Multi-dog comparison reports
- [ ] Enriched dashboard with visual stats cards
- [ ] **Testing:** Aggregation query correctness; chart data shape contract test

---

## Phase 5 — Polish & Mobile
- [ ] Touch-optimised interactions (star ratings, sliders, swipe gestures)
- [ ] Better error UX (contextual messages, retry buttons)
- [ ] PDF export for training/nutrition plans
- [ ] Basic offline support (service worker, cached last plan)
- [ ] Retry logic with exponential backoff for Anthropic calls
- [ ] **Testing:** Offline cache behaviour; PDF render snapshot test

---

## Technical Debt / Cross-Cutting Concerns
- [ ] Move `ANTHROPIC_API_KEY` / `ANTHROPIC_MODEL` env reads into `AppState` at startup (fail fast rather than per-request)
- [ ] Integration tests for all API routes (using `sqlx` test transactions)
- [ ] Frontend E2E tests (Playwright) for the critical path: login → onboarding → generate plan
- [ ] Rate limiting middleware (per-user request throttle)

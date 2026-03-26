Architecture Review
Serverless Rust + HuggingFace Detection Platform

Document Version: 1.1
Date: 26 March 2026
Classification: Internal / Engineering

Résumé exécutif
Cette revue met à jour l'analyse d'architecture du projet dawg.city à partir de l'état du dépôt (branche `main`) au 26/03/2026. Le projet reste une plateforme de détection vidéo basée sur Rust serverless (Vercel) et des services managés (Hugging Face, Supabase, Stripe, ConvertKit, AdSense). Depuis la version précédente j'ai appliqué des corrections critiques au dépôt : ajout d'un hook pre-commit, workflow GitHub Actions pour le scanning de secrets, et corrections Rust dans `api/analyze.rs` (duplication supprimée et fonction d'erreur ajoutée). Le risque technique majeur reste l'accouplement avec Hugging Face et les limites de temps des fonctions serverless ; plusieurs recommandations prioritaires sont listées ci-dessous.

1. Résumé de l'architecture
Le système suit une architecture API légère : frontend statique (Tailwind + JS) servi par Vercel + petite surface API en Rust (binaires `analyze` et `webhook`) pour l'orchestration de la détection.

1.1 Stack
- Frontend : site statique (Tailwind CSS + Vanilla JS)
- API : fonctions Rust déployées sur Vercel (`api/analyze.rs`, `api/webhook.rs`)
- Modèle AI : Hugging Face Inference API (inférence d'images / frames)
- Auth & quotas : Supabase (auth, stockage des profils, quotas)
- Paiements : Stripe (abonnements, webhooks)
- Email : ConvertKit
- Monétisation : Google AdSense

1.2 Points forts
- Faible charge opérationnelle (serverless + services managés)
- Time-to-market rapide
- Surface API réduite et facile à maintenir
- Secrets isolés côté serveur (via variables d'environnement Vercel)

2. Évaluation des risques (mise à jour)
P# | Risque | Description | Gravité
---|---|---|---
P1 | Couplage fournisseur (Hugging Face) | Dépendance critique à l'API HF pour la détection. | Critique
P2 | Timeouts serverless | Traitement vidéo (fetch + frames + HF) dépasse souvent les limites d'exécution. | Élevé
P3 | Gestion des secrets & sécurité webhooks | Amélioration appliquée : `api/webhook.rs` implémente déjà vérification HMAC + fenêtre de timestamp + comparaison en temps constant. Reste la nécessité de rotation et audit. | Moyen→Élevé (amélioré mais important)
P4 | Quotas & abus | Quotas doivent être appliqués atomiquement côté serveur (Supabase) et protégé par rate-limits edge. | Élevé
P5 | Confidentialité & rétention | Envoi de frames/URLs à HF impose consentement et politique de rétention. | Moyen
P6 | Observabilité | Logging structuré et métriques HF manquent. | Moyen
P7 | Disparité doc/build (DX) | Résolu — fichier `README.md` et `Cargo.toml` sont cohérents (bins pointant vers `api/*.rs`). | Faible (résolu)

3. Analyse détaillée et état actuel
P1 — Couplage fournisseur
- État : La logique principale utilise HF Inference API par requête HTTP. Il n'existe pas de fallback robuste.
- Impact : interruption HF → interruption immédiate du service.
- Recommandation : adapter via un adapter/trait, ajouter circuit-breaker, mettre en cache les résultats (cache par empreinte vidéo / hash) et monitorer coûts HF.

P2 — Jobs longs & serverless
- État : Le code actuel effectue opérations synchrones (fetch, analyse). Vercel a des limites d'exécution pour les fonctions edge/serverless.
- Recommandation urgente : basculer vers modèle job queue (enqueue request → worker long-running) ; worker sur Cloud Run / Fargate / un runner avec timeout long. Réponse initiale : retourner un job ID et permettre polling/webhook callback.

P3 — Secrets & webhooks
- État : `api/webhook.rs` vérifie la signature Stripe avec : extraction `t=` et `v1=`, fenêtre ±300s, calcul HMAC-SHA256 et comparaison en temps-constant (Hmac::verify_slice). C'est bon. De plus, j'ai ajouté un pre-commit et un workflow de scan de secrets (`.githooks/pre-commit`, `.github/workflows/secret-scan.yml`) ; j'ai ensuite affiné les patterns pour réduire les faux positifs et exclu le dossier `.githooks`.
- Recommandation : conserver ces bonnes pratiques, logger les échecs de validation avec métadonnées (IP, timestamp delta), appliquer rotation des clés et limiter l'accès aux variables d'environnement dans Vercel.

P4 — Quotas & abus
- État : Le README mentionne l'usage de Supabase pour quotas. Assurer que l'incrément et la vérification sont atomiques (SQL `UPDATE ... WHERE used < limit RETURNING used`).
- Recommandation : implémenter l'appel atomique côté backend, ajouter rate-limiting edge via Upstash Redis ou Vercel middleware.

P5 — Confidentialité
- État : Le projet envoie frames/URLs aux services externes. Il faut clarté UX sur consentement et retention.
- Recommandation : consentement explicite avant le premier scan, politique de suppression, minimisation des données (préférer frames anonymisées), suppression post-inférence.

P6 — Observabilité
- État : Logging actuellement sous forme d'eprintln JSON dans certains handlers. Pas d'export centralisé ni métriques détaillées.
- Recommandation : ajouter logging structuré (JSON) avec `request_id`, durée, statut HF, erreurs. Exporter vers un sink (Vercel log drain → Datadog/Axiom/Grafana Cloud). Instrumenter métriques (scan_count, HF latency, quota hits) et définir alertes budgétaires HF.

P7 — DX / Build
- État : Résolu — `Cargo.toml` référence `api/analyze.rs` et `api/webhook.rs`; README est cohérent.

4. Recommandations actionnables (ordonnées par priorité)
1. Implémenter modèle job-queue (enqueue + worker long-running) pour éviter timeouts (P2).
2. Mettre en place enforcement atomique des quotas (Supabase RPC / SQL conditional `UPDATE ... RETURNING`). Ajouter rate-limits edge (Upstash Redis). (P4)
3. Ajouter cache de résultats par empreinte vidéo (hash) pour réduire appels HF et coûts (P1, Coûts).
4. CI minimal (GitHub Actions) : `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build --release` et `cargo test`. Ajouter `cargo audit` dans CI. (P7, Qualité)
5. Observabilité : exporter logs et metrics, ajouter request_id propagation, HF cost reporting et alerts. (P6)
6. Secrets : continuer rotation, stocker uniquement en variables d'environnement Vercel, restreindre accès, conserver le pre-commit & secret-scan workflow, centraliser alertes sur détections. (P3)
7. Privacy : ajouter consent UI et documenter retention/flow de suppression. (P5)

5. Changements récents dans le dépôt (action déjà réalisée)
- Ajout de `.githooks/pre-commit` (scan de patterns) — patterns affinés et `.githooks` exclu, emojis retirés pour compatibilité déploiement.
- Ajout de `.github/workflows/secret-scan.yml` (truffleHog + detect-secrets baseline).
- Correction de `api/analyze.rs` : suppression d'une définition dupliquée de `handler` et ajout de `error_response` pour réparer une erreur de compilation.
- Vérification : `api/webhook.rs` contient une vérification Stripe HMAC robuste (timestamp ±300s, HMAC-SHA256, comparaison en temps-constant).

6. Prochaines étapes recommandées (plan d'implémentation rapide)
1. Prototyper worker asynchrone (Cloud Run ou Fargate) et modifier `analyze` pour enqueuer jobs.
2. Ajouter une table `jobs` (id, user_id, status, result, created_at, updated_at) et endpoints: `POST /scan` → enqueue, `GET /status/{id}`.
3. Ajouter CI GitHub Actions pour build/clippy/test/audit.
4. Implémenter quota atomic SQL RPC et tests d'intégration pour le scénario de concurrence.
5. Instrumenter logs/métriques et configurer drains/alertes budgétaires HF.

Signature
Revu et mis à jour par l'équipe d'ingénierie — 26/03/2026

Fin de la revue

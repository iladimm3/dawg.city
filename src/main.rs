use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod middleware;
mod models;
mod errors;
mod routes;
mod services;

use routes::{auth, dogs, training, nutrition};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub oauth: Arc<services::oauth::GoogleOAuth>,
    pub cookie_key: axum_extra::extract::cookie::Key,
}

impl axum::extract::FromRef<AppState> for axum_extra::extract::cookie::Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "dawg_city=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("Database migrations applied");

    // Google OAuth
    let oauth = Arc::new(services::oauth::GoogleOAuth::new()?);

    // Cookie signing key (store this secret in env in production)
    let cookie_secret = std::env::var("COOKIE_SECRET").expect("COOKIE_SECRET must be set");
    let cookie_key = axum_extra::extract::cookie::Key::from(cookie_secret.as_bytes());

    let state = AppState { db, oauth, cookie_key };

    // Router
    let app = Router::new()
        .route("/health", get(health_check))
        // Auth routes
        .nest("/auth", auth::router())
        // API routes (protected)
        .nest("/api/dogs", dogs::router(state.clone()))
        .nest("/api/training", training::router(state.clone()))
        .nest("/api/nutrition", nutrition::router(state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive()) // Tighten in production
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Dawg City running on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let version = env!("CARGO_PKG_VERSION");
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "version": version,
                "db": "connected"
            })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "degraded",
                "version": version,
                "db": "unreachable"
            })),
        )
            .into_response(),
    }
}

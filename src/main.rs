mod db;
mod middleware;
mod models;
mod routes;

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use http::StatusCode;
use std::net::SocketAddr;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

/// Redirect www → non-www and http → https for both dawg.city and dailyspend.city.
/// Must run before any other routing logic.
async fn canonical_redirect(req: Request, next: Next) -> Response {
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let scheme = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");

    // Strip www. prefix if present; preserve the canonical host for both domains
    let is_www = host.starts_with("www.");
    let canonical_host = if is_www { &host[4..] } else { host };
    let wrong_scheme = scheme != "https";

    if is_www || wrong_scheme {
        let clean_path = req
            .uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/");
        let target = format!("https://{}{}", canonical_host, clean_path);

        return (
            StatusCode::MOVED_PERMANENTLY,
            [(http::header::LOCATION, target)],
        )
            .into_response();
    }

    next.run(req).await
}

/// Rewrite paths for dailyspend.city requests so that ServeDir("static")
/// finds files under static/dailyspend/.  API and health paths are left alone.
async fn dailyspend_rewrite(mut req: Request, next: Next) -> Response {
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if host == "dailyspend.city" {
        let path = req.uri().path();
        if !path.starts_with("/api/") && path != "/health" {
            let new_path = if path == "/" {
                "/dailyspend/index.html".to_string()
            } else {
                format!("/dailyspend{}", path)
            };
            let query = req
                .uri()
                .query()
                .map(|q| format!("?{}", q))
                .unwrap_or_default();
            if let Ok(new_uri) = format!("{}{}", new_path, query).parse::<http::Uri>() {
                *req.uri_mut() = new_uri;
            }
        }
    }

    next.run(req).await
}

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    let db = db::connect().await;

    // Allow both domains as CORS origins
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            "https://dawg.city".parse().unwrap(),
            "https://dailyspend.city".parse().unwrap(),
        ]))
        .allow_methods(AllowMethods::list([
            http::Method::GET,
            http::Method::POST,
            http::Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
        ]))
        .allow_credentials(true);

    let api_routes = Router::new()
        // auth
        .route("/api/auth/discord", get(routes::auth::login))
        .route("/api/auth/callback", get(routes::auth::callback))
        .route("/api/me", get(routes::auth::me))
        .route("/api/mirrors", get(routes::auth::mirrors))
        // games
        .route("/api/games", get(routes::games::list))
        .route("/api/games/:slug", get(routes::games::get_game))
        .route("/api/games/:slug/ping", post(routes::games::ping))
        // leaderboard
        .route("/api/games/:slug/leaderboard", get(routes::leaderboard::get_leaderboard))
        .route("/api/games/:slug/score", post(routes::leaderboard::submit_score))
        // coins
        .route("/api/me/coins", get(routes::coins::balance))
        // battle pass
        .route("/api/battlepass", get(routes::battlepass::status))
        .route("/api/battlepass/claim/:tier", post(routes::battlepass::claim))
        // shop
        .route("/api/shop", get(routes::shop::list))
        .route("/api/shop/buy/:item_id", post(routes::shop::buy))
        .with_state(db)
        .layer(cors);

    let app = Router::new()
        .route("/health", get(health))
        .merge(api_routes)
        .fallback_service(ServeDir::new("static"))
        .layer(axum::middleware::from_fn(dailyspend_rewrite))
        .layer(axum::middleware::from_fn(canonical_redirect));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl+c");
    println!("shutting down");
}

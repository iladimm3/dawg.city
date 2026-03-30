mod analyze;
mod webhook;

use axum::{routing::{get, post}, Router};
use std::net::SocketAddr;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    let allowed_origin = std::env::var("ALLOWED_ORIGIN")
        .unwrap_or_else(|_| "https://dawg.city".to_string());

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(
            allowed_origin.parse().expect("invalid ALLOWED_ORIGIN"),
        ))
        .allow_methods(AllowMethods::list([
            http::Method::POST,
            http::Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
        ]));

    let api_routes = Router::new()
        .route("/api/analyze", post(analyze::handler))
        .route("/api/webhook", post(webhook::handler))
        .layer(cors);

    let app = Router::new()
        .route("/health", get(health))
        .merge(api_routes)
        .fallback_service(ServeDir::new("static"));

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

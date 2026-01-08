mod auth;
mod handlers;
mod models;
mod requests;
mod responses;
mod routes;

use std::{env, sync::Arc, time::Duration};

use auth::AppState;
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let db_host = env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let db_port = env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let db_user = env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string());
    let db_password = env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string());
    let db_name = env::var("DB_NAME").unwrap_or_else(|_| "postgres".to_string());
    let db_url = format!("postgres://{db_user}:{db_password}@{db_host}:{db_port}/{db_name}");

    let redis_host = env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let redis_port = env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
    let redis_url = format!("redis://{}:{}", redis_host, redis_port);

    let app_host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let app_port = env::var("APP_PORT").unwrap_or_else(|_| "5000".to_string());
    let app_address = format!("{}:{}", app_host, app_port);

    fmt::init();

    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let redis_client = Client::open(redis_url.as_str()).expect("Failed to create Redis client");
    let redis = Arc::new(redis_client);

    let app_state = AppState { pool, redis };

    let app = routes::create_router(app_state);
    let listener = tokio::net::TcpListener::bind(&app_address)
        .await
        .expect("Failed to bind address");

    println!("Server running on http://{app_address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
            println!("Shutting down...");
        })
        .await
        .unwrap();
}

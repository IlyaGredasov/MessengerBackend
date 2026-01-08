use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{auth::AppState, handlers};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/auth/login", post(handlers::login))
        .route("/users", post(handlers::create_user))
        .route("/users/{id}", get(handlers::get_user))
        .route("/users/{id}", delete(handlers::delete_user))
        .route("/users/{id}/password", put(handlers::change_password))
        .route("/users/{id}/login", put(handlers::change_login))
        .route("/messages", get(handlers::get_messages))
        .route("/messages", post(handlers::create_message))
        .route("/messages/{id}", get(handlers::get_message))
        .route("/messages/{id}", put(handlers::update_message))
        .route("/messages/{id}", delete(handlers::delete_message))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

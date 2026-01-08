use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn generate_token() -> String {
    Uuid::new_v4().to_string()
}

pub async fn store_token(
    redis: Arc<redis::Client>,
    user_id: i64,
    token: &str,
) -> Result<(), redis::RedisError> {
    let key = format!("session:{}", token);
    let mut conn = redis.get_multiplexed_async_connection().await?;
    let _: () = conn.set_ex(&key, user_id.to_string(), 300).await?;
    Ok(())
}

pub async fn get_user_id_from_token(
    redis: Arc<redis::Client>,
    token: &str,
) -> Result<Option<i64>, redis::RedisError> {
    if Uuid::parse_str(token).is_err() {
        return Ok(None);
    }

    let key = format!("session:{}", token);
    let mut conn = redis.get_multiplexed_async_connection().await?;
    let user_id_str: Option<String> = conn.get(&key).await?;

    match user_id_str {
        Some(id_str) => match id_str.parse::<i64>() {
            Ok(user_id) => Ok(Some(user_id)),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let computed_hash = hex::encode(hasher.finalize());
    computed_hash == hash
}

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Clone, Copy)]
pub struct AuthenticatedUser {
    pub user_id: i64,
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let app_state = AppState::from_ref(state);

        let user_id = get_user_id_from_token(app_state.redis.clone(), token)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(AuthenticatedUser { user_id })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub redis: Arc<redis::Client>,
}

impl FromRef<AppState> for sqlx::PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Arc<redis::Client> {
    fn from_ref(state: &AppState) -> Self {
        state.redis.clone()
    }
}

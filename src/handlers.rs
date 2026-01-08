use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing;

use crate::{
    auth::{
        generate_token, hash_password, store_token, verify_password, AppState, AuthenticatedUser,
    },
    requests::{
        ChangeLoginRequest, ChangePasswordRequest, CreateMessageRequest, CreateUserRequest,
        LoginRequest, UpdateMessageRequest,
    },
    responses::{LoginResponse, MessageResponse, UserResponse},
};

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE login = $1")
        .bind(&request.login)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error when fetching user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let user = user.ok_or(StatusCode::UNAUTHORIZED)?;

    let is_valid = verify_password(&request.password, &user.password_hash);

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = generate_token();
    store_token(state.redis.clone(), user.id, &token)
        .await
        .map_err(|e| {
            tracing::error!("Error storing token: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(LoginResponse { token }))
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<StatusCode, StatusCode> {
    let password_hash = hash_password(&request.password);

    let result = sqlx::query("INSERT INTO users (login, password_hash) VALUES ($1, $2)")
        .bind(&request.login)
        .bind(&password_hash)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.constraint().is_some()
                && (db_err.message().contains("login") || db_err.message().contains("unique"))
            {
                return Err(StatusCode::CONFLICT);
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_user(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<Json<UserResponse>, StatusCode> {
    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match user {
        Some(user) => Ok(Json(UserResponse::from(user))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn delete_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    if user.user_id != id {
        return Err(StatusCode::FORBIDDEN);
    }

    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_login(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
    Json(request): Json<ChangeLoginRequest>,
) -> Result<StatusCode, StatusCode> {
    if user.user_id != id {
        return Err(StatusCode::FORBIDDEN);
    }

    let result = sqlx::query("UPDATE users SET login = $1 WHERE id = $2")
        .bind(&request.new_login)
        .bind(id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() == 0 {
                return Err(StatusCode::NOT_FOUND);
            }
            Ok(StatusCode::NO_CONTENT)
        }
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.constraint().is_some()
                && (db_err.message().contains("login") || db_err.message().contains("unique"))
            {
                return Err(StatusCode::CONFLICT);
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn change_password(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<StatusCode, StatusCode> {
    if user.user_id != id {
        return Err(StatusCode::FORBIDDEN);
    }

    let db_user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db_user = db_user.ok_or(StatusCode::NOT_FOUND)?;

    let is_valid = verify_password(&request.old_password, &db_user.password_hash);

    if !is_valid {
        return Err(StatusCode::FORBIDDEN);
    }

    let new_password_hash = hash_password(&request.new_password);

    let result = sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&new_password_hash)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

pub async fn get_messages(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(params): Query<MessageQuery>,
) -> Result<Json<Vec<MessageResponse>>, StatusCode> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let messages = sqlx::query_as::<_, crate::models::Message>(
        "SELECT * FROM messages ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        messages.into_iter().map(MessageResponse::from).collect(),
    ))
}

pub async fn create_message(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateMessageRequest>,
) -> Result<StatusCode, StatusCode> {
    if user.user_id != request.user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query("INSERT INTO messages (user_id, text) VALUES ($1, $2)")
        .bind(request.user_id)
        .bind(request.text)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

pub async fn get_message(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<Json<MessageResponse>, StatusCode> {
    let message =
        sqlx::query_as::<_, crate::models::Message>("SELECT * FROM messages WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match message {
        Some(message) => Ok(Json(MessageResponse::from(message))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn update_message(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
    Json(request): Json<UpdateMessageRequest>,
) -> Result<StatusCode, StatusCode> {
    let message =
        sqlx::query_as::<_, crate::models::Message>("SELECT * FROM messages WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let message = message.ok_or(StatusCode::NOT_FOUND)?;

    if message.user_id != user.user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let result = sqlx::query("UPDATE messages SET text = $1 WHERE id = $2")
        .bind(request.text)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_message(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let message =
        sqlx::query_as::<_, crate::models::Message>("SELECT * FROM messages WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let message = message.ok_or(StatusCode::NOT_FOUND)?;

    if message.user_id != user.user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let result = sqlx::query("DELETE FROM messages WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

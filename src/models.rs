use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub password_hash: String,
}

#[derive(FromRow, Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub user_id: i64,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

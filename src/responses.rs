use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::models::{Message, User};

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub login: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            login: user.login,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: i64,
    pub user_id: i64,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

impl From<Message> for MessageResponse {
    fn from(message: Message) -> Self {
        MessageResponse {
            id: message.id,
            user_id: message.user_id,
            text: message.text,
            created_at: message.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

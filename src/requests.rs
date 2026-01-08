use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangeLoginRequest {
    pub new_login: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    pub user_id: i64,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMessageRequest {
    pub text: String,
}

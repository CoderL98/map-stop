use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AppState, AuthUser, hash_password, issue_token, verify_password};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct RegisterReq {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginReq {
    /// username or email
    pub login: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResp {
    pub token: String,
    pub user: UserDto,
}

#[derive(Serialize)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterReq>,
) -> Result<Json<AuthResp>, AppError> {
    let username = body.username.trim().to_string();
    if username.len() < 3 {
        return Err(AppError::bad_request("用户名至少 3 个字符"));
    }
    if body.password.len() < 6 {
        return Err(AppError::bad_request("密码至少 6 个字符"));
    }
    let email = body
        .email
        .as_ref()
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty());

    let exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ? OR (? IS NOT NULL AND email = ?)")
            .bind(&username)
            .bind(&email)
            .bind(&email)
            .fetch_optional(&state.pool)
            .await?;
    if exists.is_some() {
        return Err(AppError::conflict("用户名或邮箱已存在"));
    }

    let id = Uuid::new_v4().to_string();
    let hash = hash_password(&body.password)?;
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role) VALUES (?, ?, ?, ?, 'user')",
    )
    .bind(&id)
    .bind(&username)
    .bind(&email)
    .bind(&hash)
    .execute(&state.pool)
    .await?;

    let token = issue_token(&id, &username, "user", &state.jwt_secret)?;
    Ok(Json(AuthResp {
        token,
        user: UserDto {
            id,
            username,
            email,
            role: "user".into(),
        },
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginReq>,
) -> Result<Json<AuthResp>, AppError> {
    let login = body.login.trim();
    let row: Option<(String, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT id, username, email, password_hash, role FROM users WHERE username = ? OR email = ?",
    )
    .bind(login)
    .bind(login)
    .fetch_optional(&state.pool)
    .await?;

    let Some((id, username, email, hash, role)) = row else {
        return Err(AppError::unauthorized("用户名/邮箱或密码错误"));
    };
    if !verify_password(&body.password, &hash)? {
        return Err(AppError::unauthorized("用户名/邮箱或密码错误"));
    }
    let token = issue_token(&id, &username, &role, &state.jwt_secret)?;
    Ok(Json(AuthResp {
        token,
        user: UserDto {
            id,
            username,
            email,
            role,
        },
    }))
}

pub async fn me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<UserDto>, AppError> {
    let row: Option<(String, String, Option<String>, String)> =
        sqlx::query_as("SELECT id, username, email, role FROM users WHERE id = ?")
            .bind(&user.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some((id, username, email, role)) = row else {
        return Err(AppError::unauthorized("用户不存在"));
    };
    Ok(Json(UserDto {
        id,
        username,
        email,
        role,
    }))
}

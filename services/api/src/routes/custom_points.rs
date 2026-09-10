use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AppState, AuthUser};
use crate::error::AppError;

#[derive(Serialize)]
pub struct CustomPointDto {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: f64,
    pub note: Option<String>,
    pub selected: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct UpsertCustom {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: Option<f64>,
    pub note: Option<String>,
    pub selected: Option<bool>,
}

fn validate(lat: f64, lon: f64, radius_m: f64) -> Result<(), AppError> {
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(AppError::bad_request("经纬度超出范围"));
    }
    if radius_m <= 0.0 || radius_m > 50_000.0 {
        return Err(AppError::bad_request("半径必须大于 0 且不超过 50000 米"));
    }
    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<CustomPointDto>>, AppError> {
    let rows: Vec<(
        String,
        String,
        f64,
        f64,
        f64,
        Option<String>,
        i64,
        String,
        String,
    )> = sqlx::query_as(
        "SELECT id, name, lat, lon, radius_m, note, selected, created_at, updated_at FROM custom_points WHERE user_id=? ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(id, name, lat, lon, radius_m, note, selected, created_at, updated_at)| {
                    CustomPointDto {
                        id,
                        name,
                        lat,
                        lon,
                        radius_m,
                        note,
                        selected: selected != 0,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect(),
    ))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpsertCustom>,
) -> Result<Json<CustomPointDto>, AppError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("名称必填"));
    }
    let radius = body.radius_m.unwrap_or(80.0);
    validate(body.lat, body.lon, radius)?;
    let selected = body.selected.unwrap_or(true);
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO custom_points (id, user_id, name, lat, lon, radius_m, note, selected) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&name)
    .bind(body.lat)
    .bind(body.lon)
    .bind(radius)
    .bind(&body.note)
    .bind(if selected { 1 } else { 0 })
    .execute(&state.pool)
    .await?;
    fetch_one(&state, &user.id, &id).await
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpsertCustom>,
) -> Result<Json<CustomPointDto>, AppError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("名称必填"));
    }
    let radius = body.radius_m.unwrap_or(80.0);
    validate(body.lat, body.lon, radius)?;
    let selected = body.selected.unwrap_or(true);
    let r = sqlx::query(
        "UPDATE custom_points SET name=?, lat=?, lon=?, radius_m=?, note=?, selected=?, updated_at=datetime('now') WHERE id=? AND user_id=?",
    )
    .bind(&name)
    .bind(body.lat)
    .bind(body.lon)
    .bind(radius)
    .bind(&body.note)
    .bind(if selected { 1 } else { 0 })
    .bind(&id)
    .bind(&user.id)
    .execute(&state.pool)
    .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::not_found("自定义点不存在"));
    }
    fetch_one(&state, &user.id, &id).await
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let r = sqlx::query("DELETE FROM custom_points WHERE id=? AND user_id=?")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.pool)
        .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::not_found("自定义点不存在"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn fetch_one(
    state: &AppState,
    user_id: &str,
    id: &str,
) -> Result<Json<CustomPointDto>, AppError> {
    let row: Option<(
        String,
        String,
        f64,
        f64,
        f64,
        Option<String>,
        i64,
        String,
        String,
    )> = sqlx::query_as(
        "SELECT id, name, lat, lon, radius_m, note, selected, created_at, updated_at FROM custom_points WHERE id=? AND user_id=?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((id, name, lat, lon, radius_m, note, selected, created_at, updated_at)) = row else {
        return Err(AppError::not_found("自定义点不存在"));
    };
    Ok(Json(CustomPointDto {
        id,
        name,
        lat,
        lon,
        radius_m,
        note,
        selected: selected != 0,
        created_at,
        updated_at,
    }))
}

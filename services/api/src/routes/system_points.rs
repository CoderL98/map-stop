use axum::{
    Json,
    extract::{Multipart, Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AdminUser, AppState, AuthUser};
use crate::error::AppError;

#[derive(Serialize)]
pub struct PointDto {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: f64,
    pub note: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct UpsertPoint {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: Option<f64>,
    pub note: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct EnabledBody {
    pub enabled: bool,
}

fn validate_coords(lat: f64, lon: f64, radius_m: f64) -> Result<(), AppError> {
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
    _user: AuthUser,
) -> Result<Json<Vec<PointDto>>, AppError> {
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
        "SELECT id, name, lat, lon, radius_m, note, enabled, created_at, updated_at FROM system_points ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, name, lat, lon, radius_m, note, enabled, created_at, updated_at)| {
                PointDto {
                    id,
                    name,
                    lat,
                    lon,
                    radius_m,
                    note,
                    enabled: enabled != 0,
                    created_at,
                    updated_at,
                }
            })
            .collect(),
    ))
}

pub async fn create(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(body): Json<UpsertPoint>,
) -> Result<Json<PointDto>, AppError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("名称必填"));
    }
    let radius = body.radius_m.unwrap_or(80.0);
    validate_coords(body.lat, body.lon, radius)?;
    let enabled = body.enabled.unwrap_or(true);
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO system_points (id, name, lat, lon, radius_m, note, enabled, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(body.lat)
    .bind(body.lon)
    .bind(radius)
    .bind(&body.note)
    .bind(if enabled { 1 } else { 0 })
    .bind(&admin.0.id)
    .execute(&state.pool)
    .await?;

    fetch_one(&state, &id).await
}

pub async fn update(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpsertPoint>,
) -> Result<Json<PointDto>, AppError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("名称必填"));
    }
    let radius = body.radius_m.unwrap_or(80.0);
    validate_coords(body.lat, body.lon, radius)?;
    let enabled = body.enabled.unwrap_or(true);
    let r = sqlx::query(
        "UPDATE system_points SET name=?, lat=?, lon=?, radius_m=?, note=?, enabled=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(&name)
    .bind(body.lat)
    .bind(body.lon)
    .bind(radius)
    .bind(&body.note)
    .bind(if enabled { 1 } else { 0 })
    .bind(&id)
    .execute(&state.pool)
    .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::not_found("系统点不存在"));
    }
    fetch_one(&state, &id).await
}

pub async fn delete(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let r = sqlx::query("DELETE FROM system_points WHERE id=?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::not_found("系统点不存在"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn set_enabled(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<EnabledBody>,
) -> Result<Json<PointDto>, AppError> {
    let r = sqlx::query(
        "UPDATE system_points SET enabled=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(if body.enabled { 1 } else { 0 })
    .bind(&id)
    .execute(&state.pool)
    .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::not_found("系统点不存在"));
    }
    fetch_one(&state, &id).await
}

pub async fn import_csv(
    State(state): State<AppState>,
    admin: AdminUser,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("上传失败: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "csv" || bytes.is_none() {
            bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取文件失败: {e}")))?
                    .to_vec(),
            );
        }
    }
    let bytes = bytes.ok_or_else(|| AppError::bad_request("请上传 CSV 文件"))?;
    let text = String::from_utf8_lossy(&bytes);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers = rdr
        .headers()
        .map_err(|e| AppError::bad_request(format!("CSV 表头错误: {e}")))?
        .clone();

    // Support Chinese headers: 名称,纬度,经度,半径米,备注
    let col = |names: &[&str]| -> Option<usize> {
        headers.iter().position(|h| {
            let h = h.trim();
            names.iter().any(|n| *n == h)
        })
    };
    let i_name = col(&["名称", "name"]).ok_or_else(|| AppError::bad_request("缺少列：名称"))?;
    let i_lat = col(&["纬度", "lat"]).ok_or_else(|| AppError::bad_request("缺少列：纬度"))?;
    let i_lon = col(&["经度", "lon", "lng"]).ok_or_else(|| AppError::bad_request("缺少列：经度"))?;
    let i_r = col(&["半径米", "radius", "radius_m"]);
    let i_note = col(&["备注", "note"]);

    let mut imported = 0u32;
    let mut errors = Vec::new();
    for (idx, rec) in rdr.records().enumerate() {
        let line = idx + 2;
        let rec = match rec {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("第{line}行: {e}"));
                continue;
            }
        };
        let name = rec.get(i_name).unwrap_or("").trim().to_string();
        if name.is_empty() {
            errors.push(format!("第{line}行: 名称为空"));
            continue;
        }
        let lat: f64 = match rec.get(i_lat).unwrap_or("").trim().parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(format!("第{line}行: 纬度无效"));
                continue;
            }
        };
        let lon: f64 = match rec.get(i_lon).unwrap_or("").trim().parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(format!("第{line}行: 经度无效"));
                continue;
            }
        };
        let radius: f64 = i_r
            .and_then(|i| rec.get(i))
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(80.0);
        if let Err(e) = validate_coords(lat, lon, radius) {
            errors.push(format!("第{line}行: {}", e.message));
            continue;
        }
        let note = i_note
            .and_then(|i| rec.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO system_points (id, name, lat, lon, radius_m, note, enabled, created_by) VALUES (?, ?, ?, ?, ?, ?, 1, ?)",
        )
        .bind(&id)
        .bind(&name)
        .bind(lat)
        .bind(lon)
        .bind(radius)
        .bind(&note)
        .bind(&admin.0.id)
        .execute(&state.pool)
        .await?;
        imported += 1;
    }

    Ok(Json(serde_json::json!({
        "imported": imported,
        "errors": errors,
    })))
}

async fn fetch_one(state: &AppState, id: &str) -> Result<Json<PointDto>, AppError> {
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
        "SELECT id, name, lat, lon, radius_m, note, enabled, created_at, updated_at FROM system_points WHERE id=?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((id, name, lat, lon, radius_m, note, enabled, created_at, updated_at)) = row else {
        return Err(AppError::not_found("系统点不存在"));
    };
    Ok(Json(PointDto {
        id,
        name,
        lat,
        lon,
        radius_m,
        note,
        enabled: enabled != 0,
        created_at,
        updated_at,
    }))
}

use axum::{
    Json,
    extract::{Multipart, Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AdminUser, AppState, AuthUser};
use crate::error::AppError;

#[derive(Serialize)]
pub struct UploadDto {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: f64,
    pub note: Option<String>,
    pub status: String,
    pub reject_reason: Option<String>,
    pub created_at: String,
    pub username: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateUpload {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: Option<f64>,
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct RejectBody {
    pub reason: String,
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

pub async fn list_mine(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<UploadDto>>, AppError> {
    let rows: Vec<(
        String,
        String,
        f64,
        f64,
        f64,
        Option<String>,
        String,
        Option<String>,
        String,
    )> = sqlx::query_as(
        "SELECT id, name, lat, lon, radius_m, note, status, reject_reason, created_at FROM uploads WHERE user_id=? ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(id, name, lat, lon, radius_m, note, status, reject_reason, created_at)| {
                    UploadDto {
                        id,
                        name,
                        lat,
                        lon,
                        radius_m,
                        note,
                        status,
                        reject_reason,
                        created_at,
                        username: None,
                    }
                },
            )
            .collect(),
    ))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateUpload>,
) -> Result<Json<UploadDto>, AppError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("名称必填"));
    }
    let radius = body.radius_m.unwrap_or(80.0);
    validate(body.lat, body.lon, radius)?;
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO uploads (id, user_id, name, lat, lon, radius_m, note, status) VALUES (?, ?, ?, ?, ?, ?, ?, 'pending')",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&name)
    .bind(body.lat)
    .bind(body.lon)
    .bind(radius)
    .bind(&body.note)
    .execute(&state.pool)
    .await?;
    Ok(Json(UploadDto {
        id,
        name,
        lat: body.lat,
        lon: body.lon,
        radius_m: radius,
        note: body.note,
        status: "pending".into(),
        reject_reason: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        username: None,
    }))
}

pub async fn import_csv(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("上传失败: {e}")))?
    {
        bytes = Some(
            field
                .bytes()
                .await
                .map_err(|e| AppError::bad_request(format!("读取文件失败: {e}")))?
                .to_vec(),
        );
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
    let col = |names: &[&str]| -> Option<usize> {
        headers.iter().position(|h| names.iter().any(|n| *n == h.trim()))
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
        if let Err(e) = validate(lat, lon, radius) {
            errors.push(format!("第{line}行: {}", e.message));
            continue;
        }
        let note = i_note
            .and_then(|i| rec.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO uploads (id, user_id, name, lat, lon, radius_m, note, status) VALUES (?, ?, ?, ?, ?, ?, ?, 'pending')",
        )
        .bind(&id)
        .bind(&user.id)
        .bind(&name)
        .bind(lat)
        .bind(lon)
        .bind(radius)
        .bind(&note)
        .execute(&state.pool)
        .await?;
        imported += 1;
    }
    Ok(Json(serde_json::json!({ "imported": imported, "errors": errors })))
}

pub async fn list_pending(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<UploadDto>>, AppError> {
    let rows: Vec<(
        String,
        String,
        f64,
        f64,
        f64,
        Option<String>,
        String,
        Option<String>,
        String,
        String,
    )> = sqlx::query_as(
        r#"SELECT u.id, u.name, u.lat, u.lon, u.radius_m, u.note, u.status, u.reject_reason, u.created_at, us.username
           FROM uploads u JOIN users us ON us.id = u.user_id
           WHERE u.status = 'pending' ORDER BY u.created_at ASC"#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(id, name, lat, lon, radius_m, note, status, reject_reason, created_at, username)| {
                    UploadDto {
                        id,
                        name,
                        lat,
                        lon,
                        radius_m,
                        note,
                        status,
                        reject_reason,
                        created_at,
                        username: Some(username),
                    }
                },
            )
            .collect(),
    ))
}

pub async fn approve(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row: Option<(String, f64, f64, f64, Option<String>)> = sqlx::query_as(
        "SELECT name, lat, lon, radius_m, note FROM uploads WHERE id=? AND status='pending'",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((name, lat, lon, radius_m, note)) = row else {
        return Err(AppError::not_found("待审上传不存在"));
    };
    let sys_id = Uuid::new_v4().to_string();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO system_points (id, name, lat, lon, radius_m, note, enabled, created_by) VALUES (?, ?, ?, ?, ?, ?, 1, ?)",
    )
    .bind(&sys_id)
    .bind(&name)
    .bind(lat)
    .bind(lon)
    .bind(radius_m)
    .bind(&note)
    .bind(&admin.0.id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE uploads SET status='approved', reviewed_by=?, reviewed_at=datetime('now') WHERE id=?",
    )
    .bind(&admin.0.id)
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "ok": true, "system_point_id": sys_id })))
}

pub async fn reject(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<RejectBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let reason = body.reason.trim().to_string();
    if reason.is_empty() {
        return Err(AppError::bad_request("驳回须填写原因"));
    }
    let r = sqlx::query(
        "UPDATE uploads SET status='rejected', reject_reason=?, reviewed_by=?, reviewed_at=datetime('now') WHERE id=? AND status='pending'",
    )
    .bind(&reason)
    .bind(&admin.0.id)
    .bind(&id)
    .execute(&state.pool)
    .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::not_found("待审上传不存在"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

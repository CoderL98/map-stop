use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AdminUser, AppState, AuthUser};
use crate::csv_import::parse_avoid_points_csv;
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
    pub reviewed_at: Option<String>,
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

#[derive(Deserialize)]
pub struct AdminListQuery {
    /// pending | approved | rejected | all (default pending for backward compat)
    pub status: Option<String>,
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

fn map_mine_row(
    id: String,
    name: String,
    lat: f64,
    lon: f64,
    radius_m: f64,
    note: Option<String>,
    status: String,
    reject_reason: Option<String>,
    created_at: String,
    reviewed_at: Option<String>,
) -> UploadDto {
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
        reviewed_at,
        username: None,
    }
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
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, name, lat, lon, radius_m, note, status, reject_reason, created_at, reviewed_at FROM uploads WHERE user_id=? ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    id,
                    name,
                    lat,
                    lon,
                    radius_m,
                    note,
                    status,
                    reject_reason,
                    created_at,
                    reviewed_at,
                )| {
                    map_mine_row(
                        id,
                        name,
                        lat,
                        lon,
                        radius_m,
                        note,
                        status,
                        reject_reason,
                        created_at,
                        reviewed_at,
                    )
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
        reviewed_at: None,
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
    let parsed = parse_avoid_points_csv(&bytes)?;

    let mut imported = 0u32;
    let mut errors = parsed.errors;
    for row in parsed.rows {
        let id = Uuid::new_v4().to_string();
        match sqlx::query(
            "INSERT INTO uploads (id, user_id, name, lat, lon, radius_m, note, status) VALUES (?, ?, ?, ?, ?, ?, ?, 'pending')",
        )
        .bind(&id)
        .bind(&user.id)
        .bind(&row.name)
        .bind(row.lat)
        .bind(row.lon)
        .bind(row.radius_m)
        .bind(&row.note)
        .execute(&state.pool)
        .await
        {
            Ok(_) => imported += 1,
            Err(e) => {
                tracing::error!("upload import insert failed line {}: {e}", row.line);
                errors.push(crate::csv_import::RowError {
                    line: row.line,
                    message: "写入数据库失败".into(),
                });
            }
        }
    }
    Ok(Json(serde_json::json!({ "imported": imported, "errors": errors })))
}

pub async fn list_admin(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<AdminListQuery>,
) -> Result<Json<Vec<UploadDto>>, AppError> {
    let status = q
        .status
        .as_deref()
        .unwrap_or("pending")
        .trim()
        .to_ascii_lowercase();
    let status = match status.as_str() {
        "pending" | "approved" | "rejected" | "all" => status,
        "" => "pending".into(),
        _ => {
            return Err(AppError::bad_request(
                "status 须为 pending / approved / rejected / all",
            ));
        }
    };

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
        Option<String>,
        String,
    )> = if status == "all" {
        sqlx::query_as(
            r#"SELECT u.id, u.name, u.lat, u.lon, u.radius_m, u.note, u.status, u.reject_reason, u.created_at, u.reviewed_at, us.username
               FROM uploads u JOIN users us ON us.id = u.user_id
               ORDER BY u.created_at DESC"#,
        )
        .fetch_all(&state.pool)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT u.id, u.name, u.lat, u.lon, u.radius_m, u.note, u.status, u.reject_reason, u.created_at, u.reviewed_at, us.username
               FROM uploads u JOIN users us ON us.id = u.user_id
               WHERE u.status = ?
               ORDER BY CASE WHEN u.status = 'pending' THEN 0 ELSE 1 END, u.created_at DESC"#,
        )
        .bind(&status)
        .fetch_all(&state.pool)
        .await?
    };

    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    id,
                    name,
                    lat,
                    lon,
                    radius_m,
                    note,
                    status,
                    reject_reason,
                    created_at,
                    reviewed_at,
                    username,
                )| {
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
                        reviewed_at,
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

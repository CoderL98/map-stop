use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::auth::{AppState, AuthUser};
use crate::error::AppError;
use crate::routing::{AvoidCircle, TravelMode};

#[derive(Deserialize)]
pub struct PlanReq {
    pub start: LatLon,
    pub end: LatLon,
    pub mode: String,
    /// Optional explicit custom point ids; if omitted, use all selected custom points
    pub custom_point_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Serialize)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub type_: String,
    /// GeoJSON: [lon, lat]
    pub coordinates: Vec<[f64; 2]>,
}

#[derive(Serialize)]
pub struct PlanProps {
    pub distance_m: f64,
    pub duration_s: f64,
    pub mode: String,
    pub avoid_count: usize,
    pub polyline: Vec<[f64; 2]>, // [lat, lon] for map convenience
    pub provider: String,
    pub crs: String,
}

#[derive(Serialize)]
pub struct PlanRespOuter {
    #[serde(rename = "type")]
    type_: String,
    geometry: Geometry,
    properties: PlanProps,
}


fn validate_endpoint(label: &str, lat: f64, lon: f64) -> Result<(), AppError> {
    if !lat.is_finite() || !lon.is_finite() {
        return Err(AppError::bad_request(format!("{label}坐标无效")));
    }
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(AppError::bad_request(format!("{label}经纬度超出范围")));
    }
    Ok(())
}

pub async fn plan_route(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PlanReq>,
) -> Result<Json<PlanRespOuter>, AppError> {
    let mode = TravelMode::from_str(&body.mode)
        .ok_or_else(|| AppError::bad_request("出行方式须为 driving|walking|cycling"))?;

    validate_endpoint("起点", body.start.lat, body.start.lon)?;
    validate_endpoint("终点", body.end.lat, body.end.lon)?;
    if crate::geo::haversine_m(
        body.start.lat,
        body.start.lon,
        body.end.lat,
        body.end.lon,
    ) < 1.0
    {
        return Err(AppError::bad_request("起点与终点不能重合"));
    }

    let sys: Vec<(f64, f64, f64)> = sqlx::query_as(
        "SELECT lat, lon, radius_m FROM system_points WHERE enabled = 1",
    )
    .fetch_all(&state.pool)
    .await?;

    let custom: Vec<(f64, f64, f64)> = if let Some(ids) = &body.custom_point_ids {
        if ids.is_empty() {
            vec![]
        } else {
            let mut out = Vec::new();
            for id in ids {
                let row: Option<(f64, f64, f64)> = sqlx::query_as(
                    "SELECT lat, lon, radius_m FROM custom_points WHERE id=? AND user_id=?",
                )
                .bind(id)
                .bind(&user.id)
                .fetch_optional(&state.pool)
                .await?;
                if let Some(r) = row {
                    out.push(r);
                }
            }
            out
        }
    } else {
        sqlx::query_as(
            "SELECT lat, lon, radius_m FROM custom_points WHERE user_id=? AND selected=1",
        )
        .bind(&user.id)
        .fetch_all(&state.pool)
        .await?
    };

    let avoids: Vec<AvoidCircle> = sys
        .into_iter()
        .chain(custom.into_iter())
        .map(|(lat, lon, radius_m)| AvoidCircle {
            lat,
            lon,
            radius_m,
        })
        .collect();

    if avoids.len() > 50 {
        return Err(AppError::bad_request(
            "单次规划生效躲避点不能超过 50 个，请缩小集合或分批",
        ));
    }

    let result = state
        .routing
        .route(
            body.start.lat,
            body.start.lon,
            body.end.lat,
            body.end.lon,
            &avoids,
            mode,
        )
        .await
        .map_err(AppError::bad_request)?;

    let geo_coords: Vec<[f64; 2]> = result
        .coordinates
        .iter()
        .map(|(lat, lon)| [*lon, *lat])
        .collect();
    let polyline: Vec<[f64; 2]> = result
        .coordinates
        .iter()
        .map(|(lat, lon)| [*lat, *lon])
        .collect();

    Ok(Json(PlanRespOuter {
        type_: "Feature".into(),
        geometry: Geometry {
            type_: "LineString".into(),
            coordinates: geo_coords,
        },
        properties: PlanProps {
            distance_m: result.distance_m,
            duration_s: result.duration_s,
            mode: body.mode,
            avoid_count: avoids.len(),
            polyline,
            provider: result.provider,
            crs: state.routing.crs().into(),
        },
    }))
}

/// Primary meta endpoint for active routing provider / CRS / limits.
pub async fn routing_meta(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(state.routing.meta_json())
}

/// Backward-compatible alias (older FE used /meta/demo-bounds).
pub async fn demo_bounds(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(state.routing.meta_json())
}

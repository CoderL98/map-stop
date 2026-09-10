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
    pub polyline: Vec<[f64; 2]>, // [lat, lon] for Leaflet convenience
}

#[derive(Serialize)]
pub struct PlanRespOuter {
    #[serde(rename = "type")]
    type_: String,
    geometry: Geometry,
    properties: PlanProps,
}

pub async fn plan_route(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PlanReq>,
) -> Result<Json<PlanRespOuter>, AppError> {
    let mode = TravelMode::from_str(&body.mode)
        .ok_or_else(|| AppError::bad_request("出行方式须为 driving|walking|cycling"))?;

    // Enabled system points
    let sys: Vec<(f64, f64, f64)> = sqlx::query_as(
        "SELECT lat, lon, radius_m FROM system_points WHERE enabled = 1",
    )
    .fetch_all(&state.pool)
    .await?;

    // Selected custom points
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
        .graph
        .route(
            body.start.lat,
            body.start.lon,
            body.end.lat,
            body.end.lon,
            &avoids,
            mode,
        )
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
        },
    }))
}

pub async fn demo_bounds(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let (lat_min, lon_min, lat_max, lon_max) = state.graph.bounds();
    Json(serde_json::json!({
        "region": "杭州西湖演示路网",
        "engine": "embedded-grid-astar",
        "hard_avoid": true,
        "bounds": {
            "lat_min": lat_min,
            "lon_min": lon_min,
            "lat_max": lat_max,
            "lon_max": lon_max
        },
        "center": { "lat": 30.26, "lon": 120.15 },
        "note": "起终点请落在演示区域内；路网为网格近似，用于本地硬避开演示，非全国真实道路。"
    }))
}

//! Nominatim geocoding proxy: sets User-Agent server-side, biases to demo bounds, rate-limits.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::auth::AppState;
use crate::error::AppError;

const NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org/search";
const USER_AGENT: &str = "map-stop/0.1 (local-dev; contact: map-stop-dev)";
const MIN_INTERVAL: Duration = Duration::from_millis(1100);

#[derive(Clone, Default)]
pub struct GeocodeLimiter {
    last: Arc<Mutex<Option<Instant>>>,
}

impl GeocodeLimiter {
    pub async fn wait(&self) {
        let mut guard = self.last.lock().await;
        if let Some(prev) = *guard {
            let elapsed = prev.elapsed();
            if elapsed < MIN_INTERVAL {
                tokio::time::sleep(MIN_INTERVAL - elapsed).await;
            }
        }
        *guard = Some(Instant::now());
    }
}

#[derive(Deserialize)]
pub struct GeocodeQuery {
    pub q: String,
    /// Optional max results (1–8), default 5
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct GeocodeResult {
    pub display_name: String,
    pub lat: f64,
    pub lon: f64,
    pub name: Option<String>,
    pub typ: Option<String>,
}

#[derive(Deserialize)]
struct NominatimItem {
    lat: String,
    lon: String,
    display_name: String,
    name: Option<String>,
    #[serde(rename = "type")]
    typ: Option<String>,
}

pub async fn geocode(
    State(state): State<AppState>,
    Query(q): Query<GeocodeQuery>,
) -> Result<Json<Vec<GeocodeResult>>, AppError> {
    let query = q.q.trim();
    if query.is_empty() {
        return Err(AppError::bad_request("请输入搜索关键词"));
    }
    if query.chars().count() > 200 {
        return Err(AppError::bad_request("搜索关键词过长"));
    }
    let limit = q.limit.unwrap_or(5).clamp(1, 8);

    let (lat_min, lon_min, lat_max, lon_max) = state.graph.bounds();
    // Nominatim viewbox: left,top,right,bottom
    let viewbox = format!("{lon_min},{lat_max},{lon_max},{lat_min}");

    state.geocode_limiter.wait().await;

    let resp = state
        .http
        .get(NOMINATIM_URL)
        .query(&[
            ("q", query),
            ("format", "json"),
            ("limit", &limit.to_string()),
            ("accept-language", "zh"),
            ("viewbox", &viewbox),
            // Prefer results in viewbox but allow outside (bounded=0)
            ("bounded", "0"),
            ("addressdetails", "0"),
        ])
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("nominatim request failed: {e}");
            AppError::internal("地理编码服务暂时不可用")
        })?;

    if !resp.status().is_success() {
        tracing::warn!("nominatim status {}", resp.status());
        return Err(AppError::internal("地理编码服务返回错误"));
    }

    let items: Vec<NominatimItem> = resp.json().await.map_err(|e| {
        tracing::warn!("nominatim parse failed: {e}");
        AppError::internal("地理编码结果解析失败")
    })?;

    let out: Vec<GeocodeResult> = items
        .into_iter()
        .filter_map(|it| {
            let lat: f64 = it.lat.parse().ok()?;
            let lon: f64 = it.lon.parse().ok()?;
            Some(GeocodeResult {
                display_name: it.display_name,
                lat,
                lon,
                name: it.name,
                typ: it.typ,
            })
        })
        .collect();

    Ok(Json(out))
}

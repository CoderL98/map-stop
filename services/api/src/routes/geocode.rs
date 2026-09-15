//! Geocode proxy: Amap input tips / geo when Gaode provider active; else Nominatim.

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
use crate::routing::ProviderKind;

const NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org/search";
const AMAP_TIPS_URL: &str = "https://restapi.amap.com/v3/assistant/inputtips";
const AMAP_GEO_URL: &str = "https://restapi.amap.com/v3/geocode/geo";
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

#[derive(Deserialize)]
struct AmapTipsResp {
    status: Option<String>,
    info: Option<String>,
    #[serde(default)]
    tips: Vec<AmapTip>,
}

#[derive(Deserialize)]
struct AmapTip {
    name: Option<String>,
    district: Option<String>,
    address: Option<String>,
    /// "lon,lat" — empty for some category tips
    location: Option<String>,
    #[serde(rename = "type")]
    typ: Option<String>,
}

#[derive(Deserialize)]
struct AmapGeoResp {
    status: Option<String>,
    #[serde(default)]
    geocodes: Vec<AmapGeocode>,
}

#[derive(Deserialize)]
struct AmapGeocode {
    formatted_address: Option<String>,
    location: Option<String>,
    level: Option<String>,
}

fn parse_amap_location(loc: &str) -> Option<(f64, f64)> {
    let mut parts = loc.split(',');
    let lon: f64 = parts.next()?.parse().ok()?;
    let lat: f64 = parts.next()?.parse().ok()?;
    Some((lat, lon))
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

    if state.routing.active == ProviderKind::Gaode {
        if let Some(key) = state.routing.amap_web_key.as_deref() {
            return geocode_amap(&state, key, query, limit).await;
        }
    }

    geocode_nominatim(&state, query, limit).await
}

async fn geocode_amap(
    state: &AppState,
    key: &str,
    query: &str,
    limit: u32,
) -> Result<Json<Vec<GeocodeResult>>, AppError> {
    let resp = state
        .http
        .get(AMAP_TIPS_URL)
        .query(&[
            ("key", key),
            ("keywords", query),
            ("city", "杭州"),
            ("citylimit", "false"),
            ("datatype", "all"),
            ("output", "json"),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("amap tips failed: {e}");
            AppError::internal("高德输入提示暂时不可用")
        })?;

    if !resp.status().is_success() {
        return Err(AppError::internal("高德输入提示返回错误"));
    }

    let body: AmapTipsResp = resp.json().await.map_err(|e| {
        tracing::warn!("amap tips parse: {e}");
        AppError::internal("高德输入提示解析失败")
    })?;

    if body.status.as_deref() != Some("1") {
        tracing::warn!("amap tips status info={:?}", body.info);
        // fall through to geo
    }

    let mut out: Vec<GeocodeResult> = Vec::new();
    for tip in body.tips {
        let loc = tip.location.as_deref().unwrap_or("");
        if loc.is_empty() || loc == "[]" {
            continue;
        }
        let Some((lat, lon)) = parse_amap_location(loc) else {
            continue;
        };
        let name = tip.name.clone().unwrap_or_default();
        let district = tip.district.unwrap_or_default();
        let address = tip.address.unwrap_or_default();
        let display = [district.as_str(), address.as_str(), name.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty() && *s != "[]")
            .collect::<Vec<_>>()
            .join(" ");
        out.push(GeocodeResult {
            display_name: if display.is_empty() {
                name.clone()
            } else {
                display
            },
            lat,
            lon,
            name: tip.name,
            typ: tip.typ,
        });
        if out.len() as u32 >= limit {
            break;
        }
    }

    if out.is_empty() {
        // Fallback: geocode/geo
        let resp = state
            .http
            .get(AMAP_GEO_URL)
            .query(&[
                ("key", key),
                ("address", query),
                ("city", "杭州"),
                ("output", "json"),
            ])
            .send()
            .await
            .map_err(|_| AppError::internal("高德地理编码暂时不可用"))?;
        let body: AmapGeoResp = resp
            .json()
            .await
            .map_err(|_| AppError::internal("高德地理编码解析失败"))?;
        if body.status.as_deref() == Some("1") {
            for g in body.geocodes.into_iter().take(limit as usize) {
                let Some(loc) = g.location.as_deref() else {
                    continue;
                };
                let Some((lat, lon)) = parse_amap_location(loc) else {
                    continue;
                };
                out.push(GeocodeResult {
                    display_name: g.formatted_address.clone().unwrap_or_else(|| query.into()),
                    lat,
                    lon,
                    name: g.formatted_address,
                    typ: g.level,
                });
            }
        }
    }

    Ok(Json(out))
}

async fn geocode_nominatim(
    state: &AppState,
    query: &str,
    limit: u32,
) -> Result<Json<Vec<GeocodeResult>>, AppError> {
    let (lat_min, lon_min, lat_max, lon_max) = state.routing.embedded.bounds();
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

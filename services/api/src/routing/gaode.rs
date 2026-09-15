//! 高德 Web 服务路径规划（驾车/步行/骑行）。
//!
//! - 坐标：GCJ-02（经度在前）
//! - 驾车：`avoidpolygons` 原生避让 + 服务端硬校验
//! - 步行/骑行：官方文档未提供 avoidpolygons，仅做服务端硬校验（失败则中文报错，不静默穿行）

use crate::geo::{haversine_m, path_length_m, validate_polyline_vs_circles};
use crate::routing::{AvoidCircle, RouteResult, TravelMode};
use serde::Deserialize;

const DRIVING_URL: &str = "https://restapi.amap.com/v5/direction/driving";
const WALKING_URL: &str = "https://restapi.amap.com/v5/direction/walking";
const BICYCLING_URL: &str = "https://restapi.amap.com/v5/direction/bicycling";

/// Approx circle as regular N-gon in lon,lat for avoidpolygons.
/// Format per polygon: `lon,lat;lon,lat;...` (closed ring not required by docs).
pub fn circle_to_avoid_polygon(lat: f64, lon: f64, radius_m: f64, n: usize) -> String {
    let n = n.clamp(8, 16);
    // meters → degrees (local)
    let dlat = radius_m / 110_540.0;
    let dlon = radius_m / (111_320.0 * lat.to_radians().cos().max(0.2));
    let mut parts = Vec::with_capacity(n);
    for i in 0..n {
        let ang = (i as f64) * std::f64::consts::TAU / (n as f64);
        let plat = lat + dlat * ang.sin();
        let plon = lon + dlon * ang.cos();
        parts.push(format!("{:.6},{:.6}", plon, plat));
    }
    parts.join(";")
}

pub fn build_avoidpolygons(avoids: &[AvoidCircle]) -> String {
    // Gaode: max 32 regions
    avoids
        .iter()
        .take(32)
        .map(|c| circle_to_avoid_polygon(c.lat, c.lon, c.radius_m, 12))
        .collect::<Vec<_>>()
        .join("|")
}

#[derive(Debug, Deserialize)]
struct GaodeResp {
    status: Option<String>,
    info: Option<String>,
    #[serde(default)]
    route: Option<GaodeRoute>,
}

#[derive(Debug, Deserialize)]
struct GaodeRoute {
    #[serde(default)]
    paths: Vec<GaodePath>,
}

#[derive(Debug, Deserialize)]
struct GaodePath {
    distance: Option<serde_json::Value>,
    #[serde(default)]
    cost: Option<GaodeCost>,
    #[serde(default)]
    steps: Vec<GaodeStep>,
    /// Path-level polyline when requested via show_fields (may be absent)
    polyline: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GaodeCost {
    duration: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GaodeStep {
    polyline: Option<String>,
}

fn parse_num(v: &Option<serde_json::Value>) -> Option<f64> {
    match v {
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

/// Parse Amap polyline: points separated by `;`, each `lon,lat`.
/// Some responses use `,` between lon/lat and `;` between points.
fn parse_polyline(s: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for pt in s.split(';') {
        let pt = pt.trim();
        if pt.is_empty() {
            continue;
        }
        let mut parts = pt.split(',');
        let lon: f64 = match parts.next().and_then(|x| x.parse().ok()) {
            Some(v) => v,
            None => continue,
        };
        let lat: f64 = match parts.next().and_then(|x| x.parse().ok()) {
            Some(v) => v,
            None => continue,
        };
        out.push((lat, lon));
    }
    out
}

fn collect_path_coords(path: &GaodePath) -> Vec<(f64, f64)> {
    if let Some(ref pl) = path.polyline {
        let coords = parse_polyline(pl);
        if coords.len() >= 2 {
            return coords;
        }
    }
    let mut coords: Vec<(f64, f64)> = Vec::new();
    for step in &path.steps {
        if let Some(ref pl) = step.polyline {
            let mut step_coords = parse_polyline(pl);
            if !coords.is_empty() && !step_coords.is_empty() {
                // drop duplicate joint
                if haversine_m(
                    coords[coords.len() - 1].0,
                    coords[coords.len() - 1].1,
                    step_coords[0].0,
                    step_coords[0].1,
                ) < 1.0
                {
                    step_coords.remove(0);
                }
            }
            coords.extend(step_coords);
        }
    }
    coords
}

pub async fn route(
    http: &reqwest::Client,
    key: &str,
    start_lat: f64,
    start_lon: f64,
    end_lat: f64,
    end_lon: f64,
    avoids: &[AvoidCircle],
    mode: TravelMode,
) -> Result<RouteResult, String> {
    for c in avoids {
        if haversine_m(start_lat, start_lon, c.lat, c.lon) < c.radius_m {
            return Err("起点位于躲避圆内，请调整起点或半径".into());
        }
        if haversine_m(end_lat, end_lon, c.lat, c.lon) < c.radius_m {
            return Err("终点位于躲避圆内，请调整终点或半径".into());
        }
    }

    let (url, supports_avoid) = match mode {
        TravelMode::Driving => (DRIVING_URL, true),
        TravelMode::Walking => (WALKING_URL, false),
        TravelMode::Cycling => (BICYCLING_URL, false),
    };

    if !supports_avoid && !avoids.is_empty() {
        tracing::info!(
            "gaode {:?}: API 不支持 avoidpolygons，将仅依赖服务端硬校验",
            mode
        );
    }

    let origin = format!("{:.6},{:.6}", start_lon, start_lat);
    let destination = format!("{:.6},{:.6}", end_lon, end_lat);

    let mut req = http
        .get(url)
        .query(&[
            ("key", key),
            ("origin", origin.as_str()),
            ("destination", destination.as_str()),
            ("show_fields", "polyline,cost"),
            ("output", "json"),
        ]);

    if mode == TravelMode::Driving {
        req = req.query(&[("strategy", "32")]);
        if !avoids.is_empty() {
            let avoidpolygons = build_avoidpolygons(avoids);
            req = req.query(&[("avoidpolygons", avoidpolygons.as_str())]);
        }
    }

    let resp = req.send().await.map_err(|e| {
        tracing::warn!("gaode request failed: {e}");
        "高德路径规划请求失败，请稍后重试".to_string()
    })?;

    if !resp.status().is_success() {
        return Err(format!("高德路径规划 HTTP 错误: {}", resp.status()));
    }

    let body: GaodeResp = resp.json().await.map_err(|e| {
        tracing::warn!("gaode parse failed: {e}");
        "高德路径规划结果解析失败".to_string()
    })?;

    if body.status.as_deref() != Some("1") {
        let info = body.info.unwrap_or_else(|| "未知错误".into());
        // Common: no route / avoid too strict
        if info.contains("ENGINE_RESPONSE_DATA_ERROR")
            || info.contains("NO_ROADS")
            || info.contains("UNKNOWN_ERROR")
        {
            return Err("无法完全避开指定点位，请缩小半径或减少躲避点".into());
        }
        return Err(format!("高德算路失败: {info}"));
    }

    let path = body
        .route
        .and_then(|r| r.paths.into_iter().next())
        .ok_or_else(|| "无法完全避开指定点位，请缩小半径或减少躲避点".to_string())?;

    let mut coords = collect_path_coords(&path);
    if coords.len() < 2 {
        return Err("高德返回路线无效".into());
    }
    // Ensure start/end endpoints present
    if haversine_m(coords[0].0, coords[0].1, start_lat, start_lon) > 5.0 {
        coords.insert(0, (start_lat, start_lon));
    }
    let last = coords[coords.len() - 1];
    if haversine_m(last.0, last.1, end_lat, end_lon) > 5.0 {
        coords.push((end_lat, end_lon));
    }

    let circles: Vec<(f64, f64, f64)> = avoids
        .iter()
        .map(|c| (c.lat, c.lon, c.radius_m))
        .collect();
    // Hard-avoid validation for ALL modes (fail closed)
    if let Err(e) = validate_polyline_vs_circles(&coords, &circles) {
        return Err(if supports_avoid {
            e
        } else {
            format!(
                "{}（当前出行方式高德 API 不支持原生避让区域，已按硬约束拒绝）",
                e.trim_end_matches('。')
            )
        });
    }

    let distance_m = parse_num(&path.distance).unwrap_or_else(|| path_length_m(&coords));
    let duration_s = path
        .cost
        .as_ref()
        .and_then(|c| parse_num(&c.duration))
        .unwrap_or_else(|| distance_m / mode.speed_mps());

    Ok(RouteResult {
        coordinates: coords,
        distance_m,
        duration_s,
        provider: "gaode".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avoid_polygon_format() {
        let s = circle_to_avoid_polygon(30.26, 120.15, 100.0, 12);
        assert!(s.contains(';'));
        let pts: Vec<_> = s.split(';').collect();
        assert_eq!(pts.len(), 12);
        let first = pts[0];
        let (lon, lat) = first.split_once(',').unwrap();
        assert!(lon.parse::<f64>().unwrap() > 120.0);
        assert!(lat.parse::<f64>().unwrap() > 30.0);
    }

    #[test]
    fn parse_polyline_basic() {
        let coords = parse_polyline("120.15,30.26;120.16,30.27");
        assert_eq!(coords.len(), 2);
        assert!((coords[0].0 - 30.26).abs() < 1e-6);
        assert!((coords[0].1 - 120.15).abs() < 1e-6);
    }
}

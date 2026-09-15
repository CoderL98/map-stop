//! Pluggable routing providers: gaode (default with key), embedded, opensource stub.

mod embedded;
mod gaode;
mod opensource;

pub use embedded::RoadGraph;

use std::sync::Arc;

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TravelMode {
    Driving,
    Walking,
    Cycling,
}

impl TravelMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "driving" => Some(Self::Driving),
            "walking" => Some(Self::Walking),
            "cycling" => Some(Self::Cycling),
            _ => None,
        }
    }

    pub fn speed_mps(self) -> f64 {
        match self {
            Self::Driving => 11.0,
            Self::Walking => 1.4,
            Self::Cycling => 4.2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AvoidCircle {
    pub lat: f64,
    pub lon: f64,
    pub radius_m: f64,
}

#[derive(Clone, Debug)]
pub struct RouteResult {
    pub coordinates: Vec<(f64, f64)>, // lat, lon
    pub distance_m: f64,
    pub duration_s: f64,
    pub provider: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Gaode,
    Embedded,
    Opensource,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gaode => "gaode",
            Self::Embedded => "embedded",
            Self::Opensource => "opensource",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gaode" | "amap" => Some(Self::Gaode),
            "embedded" | "demo" => Some(Self::Embedded),
            "opensource" | "osrm" | "graphhopper" | "valhalla" => Some(Self::Opensource),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct RoutingService {
    /// Effective provider used for /plan
    pub active: ProviderKind,
    /// What env requested (may differ if fallback)
    pub requested: ProviderKind,
    pub fallback_warning: Option<String>,
    pub embedded: Arc<RoadGraph>,
    pub amap_web_key: Option<String>,
    /// JS API key exposed to frontend via /meta/routing (never commit real keys)
    pub amap_js_key: Option<String>,
    pub amap_security_js_code: Option<String>,
    pub http: reqwest::Client,
}

impl RoutingService {
    pub fn from_config(
        cfg: &crate::config::Config,
        http: reqwest::Client,
        embedded: RoadGraph,
    ) -> Self {
        let has_web_key = cfg
            .amap_web_key
            .as_ref()
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false);

        let requested = match cfg.routing_provider.as_deref() {
            Some(s) => ProviderKind::parse(s).unwrap_or_else(|| {
                tracing::warn!("unknown ROUTING_PROVIDER={s}, using auto");
                if has_web_key {
                    ProviderKind::Gaode
                } else {
                    ProviderKind::Embedded
                }
            }),
            None => {
                if has_web_key {
                    ProviderKind::Gaode
                } else {
                    ProviderKind::Embedded
                }
            }
        };

        let (active, fallback_warning) = match requested {
            ProviderKind::Gaode if !has_web_key => (
                ProviderKind::Embedded,
                Some(
                    "ROUTING_PROVIDER=gaode 但未设置 AMAP_WEB_KEY，已回退 embedded"
                        .to_string(),
                ),
            ),
            other => (other, None),
        };

        if let Some(ref w) = fallback_warning {
            tracing::warn!("{w}");
        }
        tracing::info!(
            "routing provider: active={} requested={}",
            active.as_str(),
            requested.as_str()
        );

        Self {
            active,
            requested,
            fallback_warning,
            embedded: Arc::new(embedded),
            amap_web_key: cfg.amap_web_key.clone().filter(|s| !s.trim().is_empty()),
            amap_js_key: cfg.amap_js_key.clone().filter(|s| !s.trim().is_empty()),
            amap_security_js_code: cfg
                .amap_security_js_code
                .clone()
                .filter(|s| !s.trim().is_empty()),
            http,
        }
    }

    pub fn crs(&self) -> &'static str {
        match self.active {
            ProviderKind::Gaode => "GCJ-02",
            // Demo grid uses the same numeric Hangzhou bbox; treat as GCJ-02 when migrating to Gaode.
            ProviderKind::Embedded => "GCJ-02-demo",
            ProviderKind::Opensource => "WGS84",
        }
    }

    pub fn engine_label(&self) -> &'static str {
        match self.active {
            ProviderKind::Gaode => "amap-webservice-v5",
            ProviderKind::Embedded => "embedded-grid-astar",
            ProviderKind::Opensource => "opensource-stub",
        }
    }

    pub async fn route(
        &self,
        start_lat: f64,
        start_lon: f64,
        end_lat: f64,
        end_lon: f64,
        avoids: &[AvoidCircle],
        mode: TravelMode,
    ) -> Result<RouteResult, String> {
        match self.active {
            ProviderKind::Embedded => self.embedded.route(
                start_lat, start_lon, end_lat, end_lon, avoids, mode,
            ),
            ProviderKind::Gaode => {
                let key = self
                    .amap_web_key
                    .as_deref()
                    .ok_or_else(|| "未配置 AMAP_WEB_KEY".to_string())?;
                gaode::route(
                    &self.http,
                    key,
                    start_lat,
                    start_lon,
                    end_lat,
                    end_lon,
                    avoids,
                    mode,
                )
                .await
            }
            ProviderKind::Opensource => {
                tracing::info!("opensource provider selected — stub not configured");
                opensource::route(start_lat, start_lon, end_lat, end_lon, avoids, mode).await
            }
        }
    }

    pub fn meta_json(&self) -> serde_json::Value {
        let (lat_min, lon_min, lat_max, lon_max) = self.embedded.bounds();
        let mut limitations = vec![
            "单次生效躲避点 ≤ 50".to_string(),
            "服务端对所有 provider 做折线硬避开二次校验".to_string(),
        ];
        match self.active {
            ProviderKind::Gaode => {
                limitations.push(
                    "驾车：高德 avoidpolygons 原生避让 + Rust 校验".into(),
                );
                limitations.push(
                    "步行/骑行：高德 API 无 avoidpolygons，仅 Rust 硬校验（穿行则失败）".into(),
                );
                limitations.push("坐标系 GCJ-02；请使用高德地图点选，勿混用未转换的 WGS84".into());
            }
            ProviderKind::Embedded => {
                limitations.push("演示区域：杭州西湖附近网格，非全国真实道路".into());
            }
            ProviderKind::Opensource => {
                limitations.push("开源引擎未配置，规划将返回「未配置」错误".into());
            }
        }

        serde_json::json!({
            "provider": self.active.as_str(),
            "requested_provider": self.requested.as_str(),
            "engine": self.engine_label(),
            "crs": self.crs(),
            "hard_avoid": true,
            "fallback_warning": self.fallback_warning,
            "amap_configured": self.amap_web_key.is_some(),
            "amap_js_key": self.amap_js_key,
            "amap_security_js_code": self.amap_security_js_code,
            "geocoder": match self.active {
                ProviderKind::Gaode if self.amap_web_key.is_some() => "amap",
                _ => "nominatim",
            },
            "region": match self.active {
                ProviderKind::Gaode => "全国（高德覆盖范围）",
                ProviderKind::Embedded => "杭州西湖演示路网",
                ProviderKind::Opensource => "未配置",
            },
            "bounds": {
                "lat_min": lat_min,
                "lon_min": lon_min,
                "lat_max": lat_max,
                "lon_max": lon_max
            },
            "center": { "lat": 30.26, "lon": 120.15 },
            "limitations": limitations,
            "note": match self.active {
                ProviderKind::Gaode => "v1 使用高德 Web 服务算路；坐标 GCJ-02。二期可切换 ROUTING_PROVIDER=opensource 自托管。",
                ProviderKind::Embedded => "内嵌演示路网；设置 AMAP_WEB_KEY 并 ROUTING_PROVIDER=gaode 可切换高德。",
                ProviderKind::Opensource => "开源路由接口已预留，请配置自托管引擎后实现 opensource provider。",
            },
            "switch_path": "同一 RoutingProvider 接口：ROUTING_PROVIDER=gaode|embedded|opensource"
        })
    }
}

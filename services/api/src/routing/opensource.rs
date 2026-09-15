//! Placeholder for future self-hosted open-source routing (GraphHopper / Valhalla).
//! Compiles and is selectable via ROUTING_PROVIDER=opensource.

use crate::routing::{AvoidCircle, RouteResult, TravelMode};

pub async fn route(
    _start_lat: f64,
    _start_lon: f64,
    _end_lat: f64,
    _end_lon: f64,
    _avoids: &[AvoidCircle],
    _mode: TravelMode,
) -> Result<RouteResult, String> {
    Err(
        "开源路由引擎尚未配置（预留 GraphHopper/Valhalla，二期接入同一 RoutingProvider 接口）"
            .into(),
    )
}

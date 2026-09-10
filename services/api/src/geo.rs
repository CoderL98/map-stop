/// Haversine distance in meters between two WGS84 points.
pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6_371_000.0;
    let (phi1, phi2) = (lat1.to_radians(), lat2.to_radians());
    let dphi = (lat2 - lat1).to_radians();
    let dlam = (lon2 - lon1).to_radians();
    let a = (dphi / 2.0).sin().powi(2) + phi1.cos() * phi2.cos() * (dlam / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
}

/// Minimum distance from point C to segment AB (meters, approximate local projection).
pub fn point_to_segment_m(
    lat_a: f64,
    lon_a: f64,
    lat_b: f64,
    lon_b: f64,
    lat_c: f64,
    lon_c: f64,
) -> f64 {
    // Local equirectangular meters around A
    let lat0 = lat_a.to_radians();
    let mx = |lon: f64| (lon - lon_a) * lat0.cos() * 111_320.0;
    let my = |lat: f64| (lat - lat_a) * 110_540.0;
    let ax = 0.0;
    let ay = 0.0;
    let bx = mx(lon_b);
    let by = my(lat_b);
    let cx = mx(lon_c);
    let cy = my(lat_c);
    let abx = bx - ax;
    let aby = by - ay;
    let len2 = abx * abx + aby * aby;
    if len2 < 1e-6 {
        return haversine_m(lat_a, lon_a, lat_c, lon_c);
    }
    let t = ((cx - ax) * abx + (cy - ay) * aby) / len2;
    let t = t.clamp(0.0, 1.0);
    let px = ax + t * abx;
    let py = ay + t * aby;
    ((cx - px).powi(2) + (cy - py).powi(2)).sqrt()
}

/// True if segment AB comes within `radius_m` of circle center.
pub fn segment_intersects_circle(
    lat_a: f64,
    lon_a: f64,
    lat_b: f64,
    lon_b: f64,
    lat_c: f64,
    lon_c: f64,
    radius_m: f64,
) -> bool {
    point_to_segment_m(lat_a, lon_a, lat_b, lon_b, lat_c, lon_c) < radius_m
}

/// Validate polyline does not enter any avoid circle. Returns Ok or Chinese error.
pub fn validate_polyline_vs_circles(
    coords: &[(f64, f64)], // (lat, lon)
    circles: &[(f64, f64, f64)], // (lat, lon, radius_m)
) -> Result<(), String> {
    if coords.len() < 2 {
        return Err("路线无效".into());
    }
    for w in coords.windows(2) {
        let (lat_a, lon_a) = w[0];
        let (lat_b, lon_b) = w[1];
        for &(clat, clon, r) in circles {
            // Also reject if either endpoint is inside
            if haversine_m(lat_a, lon_a, clat, clon) < r
                || haversine_m(lat_b, lon_b, clat, clon) < r
                || segment_intersects_circle(lat_a, lon_a, lat_b, lon_b, clat, clon, r)
            {
                return Err(
                    "路线校验失败：规划结果进入了躲避圆，请缩小半径或减少躲避点".into(),
                );
            }
        }
    }
    Ok(())
}

pub fn path_length_m(coords: &[(f64, f64)]) -> f64 {
    coords
        .windows(2)
        .map(|w| haversine_m(w[0].0, w[0].1, w[1].0, w[1].1))
        .sum()
}

//! Embedded demo road graph + A* with hard avoid of circular zones.
//! Demo region: Hangzhou West Lake area (approx 30.20–30.32°N, 120.08–120.22°E).

use crate::geo::{haversine_m, path_length_m, segment_intersects_circle, validate_polyline_vs_circles};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

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

    /// Nominal speed m/s for duration estimate.
    pub fn speed_mps(self) -> f64 {
        match self {
            Self::Driving => 11.0,  // ~40 km/h urban
            Self::Walking => 1.4,   // ~5 km/h
            Self::Cycling => 4.2,   // ~15 km/h
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
}

#[derive(Clone, Copy, Debug)]
struct Node {
    lat: f64,
    lon: f64,
}

pub struct RoadGraph {
    nodes: Vec<Node>,
    /// adjacency: node_idx -> list of (neighbor_idx, length_m)
    adj: Vec<Vec<(usize, f64)>>,
    cols: usize,
    rows: usize,
    lat_min: f64,
    lon_min: f64,
    dlat: f64,
    dlon: f64,
}

impl RoadGraph {
    /// Build a rectangular street grid for the Hangzhou demo bbox.
    pub fn hangzhou_demo() -> Self {
        // West Lake / downtown Hangzhou demo
        let lat_min = 30.20;
        let lat_max = 30.32;
        let lon_min = 120.08;
        let lon_max = 120.22;
        // ~90m steps
        let dlat = 0.0008;
        let dlon = 0.0009;
        let rows = f64::floor((lat_max - lat_min) / dlat) as usize + 1;
        let cols = f64::floor((lon_max - lon_min) / dlon) as usize + 1;
        let n = rows * cols;
        let mut nodes = Vec::with_capacity(n);
        for r in 0..rows {
            for c in 0..cols {
                nodes.push(Node {
                    lat: lat_min + r as f64 * dlat,
                    lon: lon_min + c as f64 * dlon,
                });
            }
        }
        let mut adj = vec![Vec::new(); n];
        let idx = |r: usize, c: usize| -> usize { r * cols + c };
        for r in 0..rows {
            for c in 0..cols {
                let i = idx(r, c);
                let a = nodes[i];
                // 4-connected grid (roads)
                let neighbors = [
                    (r.wrapping_sub(1), c),
                    (r + 1, c),
                    (r, c.wrapping_sub(1)),
                    (r, c + 1),
                ];
                for (nr, nc) in neighbors {
                    if nr >= rows || nc >= cols {
                        continue;
                    }
                    let j = idx(nr, nc);
                    let b = nodes[j];
                    let dist = haversine_m(a.lat, a.lon, b.lat, b.lon);
                    adj[i].push((j, dist));
                }
                // Occasional diagonals for more natural paths (every other cell)
                if (r + c) % 3 == 0 {
                    for (dr, dc) in [(1isize, 1), (1, -1)] {
                        let nr = r as isize + dr;
                        let nc = c as isize + dc;
                        if nr < 0 || nc < 0 || nr as usize >= rows || nc as usize >= cols {
                            continue;
                        }
                        let j = idx(nr as usize, nc as usize);
                        let b = nodes[j];
                        let dist = haversine_m(a.lat, a.lon, b.lat, b.lon);
                        adj[i].push((j, dist));
                        adj[j].push((i, dist));
                    }
                }
            }
        }
        // Punch a few "lakes / parks" holes so the graph isn't a perfect grid
        // West Lake rough hole around 30.25, 120.14
        let mut blocked = HashSet::new();
        for (i, node) in nodes.iter().enumerate() {
            let d = haversine_m(node.lat, node.lon, 30.25, 120.14);
            if d < 1200.0 {
                blocked.insert(i);
            }
        }
        for i in 0..n {
            adj[i].retain(|(j, _)| !blocked.contains(&i) && !blocked.contains(j));
        }

        Self {
            nodes,
            adj,
            cols,
            rows,
            lat_min,
            lon_min,
            dlat,
            dlon,
        }
    }

    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        (
            self.lat_min,
            self.lon_min,
            self.lat_min + (self.rows - 1) as f64 * self.dlat,
            self.lon_min + (self.cols - 1) as f64 * self.dlon,
        )
    }

    fn nearest_node(&self, lat: f64, lon: f64) -> Option<usize> {
        let mut best = None;
        let mut best_d = f64::MAX;
        for (i, n) in self.nodes.iter().enumerate() {
            if self.adj[i].is_empty() {
                continue;
            }
            let d = haversine_m(lat, lon, n.lat, n.lon);
            if d < best_d {
                best_d = d;
                best = Some(i);
            }
        }
        // Reject if too far from graph (> 2km)
        if best_d > 2000.0 {
            return None;
        }
        best
    }

    fn edge_blocked(&self, a: usize, b: usize, avoids: &[AvoidCircle]) -> bool {
        let na = &self.nodes[a];
        let nb = &self.nodes[b];
        for c in avoids {
            if haversine_m(na.lat, na.lon, c.lat, c.lon) < c.radius_m
                || haversine_m(nb.lat, nb.lon, c.lat, c.lon) < c.radius_m
                || segment_intersects_circle(
                    na.lat, na.lon, nb.lat, nb.lon, c.lat, c.lon, c.radius_m,
                )
            {
                return true;
            }
        }
        false
    }

    pub fn route(
        &self,
        start_lat: f64,
        start_lon: f64,
        end_lat: f64,
        end_lon: f64,
        avoids: &[AvoidCircle],
        mode: TravelMode,
    ) -> Result<RouteResult, String> {
        // Start/end must not be inside an avoid circle
        for c in avoids {
            if haversine_m(start_lat, start_lon, c.lat, c.lon) < c.radius_m {
                return Err("起点位于躲避圆内，请调整起点或半径".into());
            }
            if haversine_m(end_lat, end_lon, c.lat, c.lon) < c.radius_m {
                return Err("终点位于躲避圆内，请调整终点或半径".into());
            }
        }

        let start = self
            .nearest_node(start_lat, start_lon)
            .ok_or_else(|| "起点不在演示路网范围内（杭州西湖演示区）".to_string())?;
        let goal = self
            .nearest_node(end_lat, end_lon)
            .ok_or_else(|| "终点不在演示路网范围内（杭州西湖演示区）".to_string())?;

        if self.adj[start].is_empty() || self.adj[goal].is_empty() {
            return Err("起终点附近无可用道路节点".into());
        }

        #[derive(Copy, Clone)]
        struct State {
            cost: f64,
            node: usize,
        }
        impl PartialEq for State {
            fn eq(&self, other: &Self) -> bool {
                self.cost == other.cost && self.node == other.node
            }
        }
        impl Eq for State {}
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other
                    .cost
                    .partial_cmp(&self.cost)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| self.node.cmp(&other.node))
            }
        }

        let heuristic = |n: usize| {
            let a = &self.nodes[n];
            let b = &self.nodes[goal];
            haversine_m(a.lat, a.lon, b.lat, b.lon)
        };

        let mut open = BinaryHeap::new();
        open.push(State {
            cost: heuristic(start),
            node: start,
        });
        let mut g_score: HashMap<usize, f64> = HashMap::new();
        g_score.insert(start, 0.0);
        let mut came_from: HashMap<usize, usize> = HashMap::new();

        let mut found = false;
        while let Some(State { cost: _, node }) = open.pop() {
            if node == goal {
                found = true;
                break;
            }
            let g = *g_score.get(&node).unwrap_or(&f64::MAX);
            for &(nei, dist) in &self.adj[node] {
                if self.edge_blocked(node, nei, avoids) {
                    continue;
                }
                let tentative = g + dist;
                if tentative < *g_score.get(&nei).unwrap_or(&f64::MAX) {
                    came_from.insert(nei, node);
                    g_score.insert(nei, tentative);
                    open.push(State {
                        cost: tentative + heuristic(nei),
                        node: nei,
                    });
                }
            }
        }

        if !found {
            return Err(
                "无法完全避开指定点位，请缩小半径或减少躲避点".into(),
            );
        }

        // Reconstruct path (node indices)
        let mut path_idx = vec![goal];
        let mut cur = goal;
        while cur != start {
            cur = *came_from
                .get(&cur)
                .ok_or_else(|| "路径重构失败".to_string())?;
            path_idx.push(cur);
        }
        path_idx.reverse();

        let mut coords: Vec<(f64, f64)> = Vec::with_capacity(path_idx.len() + 2);
        coords.push((start_lat, start_lon));
        for &i in &path_idx {
            let n = &self.nodes[i];
            coords.push((n.lat, n.lon));
        }
        coords.push((end_lat, end_lon));

        // Deduplicate consecutive near-identical points
        coords.dedup_by(|a, b| haversine_m(a.0, a.1, b.0, b.1) < 1.0);

        let circles: Vec<(f64, f64, f64)> = avoids
            .iter()
            .map(|c| (c.lat, c.lon, c.radius_m))
            .collect();
        validate_polyline_vs_circles(&coords, &circles)?;

        let distance_m = path_length_m(&coords);
        let duration_s = distance_m / mode.speed_mps();

        Ok(RouteResult {
            coordinates: coords,
            distance_m,
            duration_s,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_without_avoid() {
        let g = RoadGraph::hangzhou_demo();
        let r = g
            .route(30.22, 120.10, 30.30, 120.20, &[], TravelMode::Driving)
            .unwrap();
        assert!(r.distance_m > 1000.0);
        assert!(r.coordinates.len() > 2);
    }

    #[test]
    fn hard_avoid_detours_or_fails() {
        let g = RoadGraph::hangzhou_demo();
        // Place a large avoid roughly on the straight path
        let avoids = vec![AvoidCircle {
            lat: 30.26,
            lon: 120.15,
            radius_m: 800.0,
        }];
        let r = g.route(30.22, 120.10, 30.30, 120.20, &avoids, TravelMode::Driving);
        // Either succeeds with detour that validates, or fails with Chinese message
        match r {
            Ok(route) => {
                let circles = [(30.26, 120.15, 800.0)];
                validate_polyline_vs_circles(&route.coordinates, &circles).unwrap();
            }
            Err(e) => assert!(e.contains("避开") || e.contains("躲避")),
        }
    }
}

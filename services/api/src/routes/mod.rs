pub mod auth;
pub mod custom_points;
pub mod geocode;
pub mod plan;
pub mod system_points;
pub mod uploads;

use axum::{
    Router,
    routing::{get, patch, post, put},
};
use crate::auth::AppState;

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/me", get(auth::me))
        .route("/system-points", get(system_points::list).post(system_points::create))
        .route("/system-points/import", post(system_points::import_csv))
        .route(
            "/system-points/{id}",
            put(system_points::update)
                .patch(system_points::update)
                .delete(system_points::delete),
        )
        .route(
            "/system-points/{id}/enabled",
            patch(system_points::set_enabled),
        )
        .route(
            "/custom-points",
            get(custom_points::list).post(custom_points::create),
        )
        .route(
            "/custom-points/{id}",
            put(custom_points::update).delete(custom_points::delete),
        )
        .route("/uploads", get(uploads::list_mine).post(uploads::create))
        .route("/uploads/import", post(uploads::import_csv))
        .route("/admin/uploads", get(uploads::list_admin))
        .route("/admin/uploads/{id}/approve", post(uploads::approve))
        .route("/admin/uploads/{id}/reject", post(uploads::reject))
        .route("/plan", post(plan::plan_route))
        .route("/meta/routing", get(plan::routing_meta))
        .route("/meta/demo-bounds", get(plan::demo_bounds))
        .route("/geocode", get(geocode::geocode))
        .with_state(state)
}

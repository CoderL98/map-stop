use crate::auth::hash_password;
use crate::config::Config;
use crate::error::AppError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use uuid::Uuid;

pub async fn init_pool(database_url: &str) -> Result<SqlitePool, AppError> {
    // Ensure parent dir exists for sqlite file
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let opts = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| AppError::internal(format!("数据库 URL 无效: {e}")))?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .map_err(|e| AppError::internal(format!("连接数据库失败: {e}")))?;

    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            email TEXT UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('user','admin')),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS system_points (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            lat REAL NOT NULL,
            lon REAL NOT NULL,
            radius_m REAL NOT NULL DEFAULT 80,
            note TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_by TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS custom_points (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            lat REAL NOT NULL,
            lon REAL NOT NULL,
            radius_m REAL NOT NULL DEFAULT 80,
            note TEXT,
            selected INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS uploads (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            lat REAL NOT NULL,
            lon REAL NOT NULL,
            radius_m REAL NOT NULL DEFAULT 80,
            note TEXT,
            status TEXT NOT NULL CHECK(status IN ('pending','approved','rejected')) DEFAULT 'pending',
            reject_reason TEXT,
            reviewed_by TEXT,
            reviewed_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn seed_admin(pool: &SqlitePool, cfg: &Config) -> Result<(), AppError> {
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ? OR role = 'admin' LIMIT 1")
            .bind(&cfg.admin_username)
            .fetch_optional(pool)
            .await?;

    if existing.is_some() {
        // Ensure named admin exists; if username taken with different role, skip
        let by_name: Option<(String,)> =
            sqlx::query_as("SELECT id FROM users WHERE username = ?")
                .bind(&cfg.admin_username)
                .fetch_optional(pool)
                .await?;
        if by_name.is_some() {
            tracing::info!("admin user already present");
            return Ok(());
        }
    }

    let by_name: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username = ?")
        .bind(&cfg.admin_username)
        .fetch_optional(pool)
        .await?;
    if by_name.is_some() {
        return Ok(());
    }

    let id = Uuid::new_v4().to_string();
    let hash = hash_password(&cfg.admin_password)?;
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role) VALUES (?, ?, NULL, ?, 'admin')",
    )
    .bind(&id)
    .bind(&cfg.admin_username)
    .bind(&hash)
    .execute(pool)
    .await?;
    tracing::info!("seeded admin user '{}'", cfg.admin_username);
    Ok(())
}

pub async fn seed_demo_points(pool: &SqlitePool) -> Result<(), AppError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM system_points")
        .fetch_one(pool)
        .await?;
    if count.0 > 0 {
        return Ok(());
    }
    let demos = [
        ("断桥附近演示禁区", 30.2585, 120.1480, 100.0, Some("首次启动示例，可删")),
        ("苏堤北口演示禁区", 30.2500, 120.1400, 120.0, Some("首次启动示例，可删")),
    ];
    for (name, lat, lon, r, note) in demos {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO system_points (id, name, lat, lon, radius_m, note, enabled) VALUES (?, ?, ?, ?, ?, ?, 1)",
        )
        .bind(&id)
        .bind(name)
        .bind(lat)
        .bind(lon)
        .bind(r)
        .bind(note)
        .execute(pool)
        .await?;
    }
    tracing::info!("seeded {} demo system points", demos.len());
    Ok(())
}

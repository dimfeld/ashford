use std::{env, net::SocketAddr};

use ashford_core::{
    Config, Database, JobQueue, NoopExecutor, WorkerConfig, init_telemetry, migrations,
};
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    let config = Config::load(&config_path)?;

    let _guard = init_telemetry(&config.app, &config.telemetry)?;

    let db = Database::new(&config.paths.database).await?;
    migrations::run_migrations(&db).await?;

    let queue = JobQueue::new(db.clone());
    let shutdown = CancellationToken::new();
    let worker_shutdown = shutdown.child_token();
    let worker_handle = tokio::spawn(ashford_core::run_worker(
        queue,
        NoopExecutor,
        WorkerConfig::default(),
        worker_shutdown,
    ));

    let state = AppState { db: db.clone() };
    let app = router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.app.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Ashford listening on {}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown.clone()))
        .await?;

    shutdown.cancel();
    let _ = worker_handle.await;
    Ok(())
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    database: String,
}

async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_status = match state.db.health_check().await {
        Ok(_) => "ok",
        Err(_) => "unhealthy",
    };

    let status = if db_status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(HealthResponse {
            status: if db_status == "ok" {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
            database: db_status.to_string(),
        }),
    )
}

async fn shutdown_signal(shutdown: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("received ctrl+c, shutting down");
        }
        _ = terminate => {
            warn!("received terminate signal, shutting down");
        }
    }

    shutdown.cancel();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn healthz_reports_ok_when_database_is_reachable() {
        let db = Database::new(std::path::Path::new(":memory:"))
            .await
            .expect("db");
        let state = AppState { db };
        let (status, Json(body)) = healthz(State(state)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.status, "healthy");
        assert_eq!(body.database, "ok");
    }
}

//! A service with Kubernetes probes over PostgreSQL, Redis and NATS, and a graceful shutdown that
//! fails readiness before it stops serving.
//!
//! ```sh
//! DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres \
//! REDIS_URL=redis://127.0.0.1:6379 NATS_URL=nats://127.0.0.1:4222 \
//!     cargo run -p sisa-health-checks-example-axum-service
//! curl -i 'http://127.0.0.1:9090/readyz?verbose'
//! ```

use std::{env, error::Error, time::Duration};

use axum::Router;
use sisa_health_checks::{
    EventLoopCheck, HealthRegistry, LIVE, NatsCheck, PostgresCheck, RedisCheck, Status, async_nats,
    probe_routes, redis, report_routes, sqlx::PgPool,
};
use tokio::{net::TcpListener, signal};

/// How long `/readyz` fails before the server stops, so load balancers and kube-proxy stop
/// sending traffic first.
const DRAIN_DELAY: Duration = Duration::from_secs(5);

fn setting(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

/// Waits for Ctrl-C or, on Unix, SIGTERM, which Kubernetes sends before it stops a pod.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut terminate) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            terminate.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let pool = PgPool::connect_lazy(&setting(
        "DATABASE_URL",
        "postgres://postgres:postgres@127.0.0.1:5432/postgres",
    ))?;
    let cache = redis::Client::open(setting("REDIS_URL", "redis://127.0.0.1:6379"))?
        .get_connection_manager()
        .await?;
    let nats = async_nats::connect(setting("NATS_URL", "nats://127.0.0.1:4222")).await?;

    let registry = HealthRegistry::builder()
        .check("event-loop", EventLoopCheck::default())
        .tags([LIVE])
        .check("postgres", PostgresCheck::new(pool))
        .check("redis", RedisCheck::new(cache))
        // A cache: the service can answer without it, more slowly.
        .failure(Status::Degraded)
        .check("nats", NatsCheck::new(nats))
        .build()?;
    let _poller = registry.spawn();

    // On the admin port only: the report shows details the public port shouldn't.
    let admin = Router::new()
        .merge(probe_routes(registry.clone()))
        .merge(report_routes(registry.clone()));
    let listener = TcpListener::bind(setting("ADMIN_ADDR", "0.0.0.0:9090")).await?;

    let draining = registry.clone();
    axum::serve(listener, admin)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            draining.begin_draining();
            tracing::info!("draining: /readyz fails until the server stops");
            tokio::time::sleep(DRAIN_DELAY).await;
        })
        .await?;

    Ok(())
}

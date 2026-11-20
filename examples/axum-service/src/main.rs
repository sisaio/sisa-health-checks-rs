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

use axum::{
    Router,
    extract::{RawQuery, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sisa_health_checks::{
    EventLoopCheck, HealthRegistry, LIVE, NatsCheck, PostgresCheck, Probe, RedisCheck, Status,
    async_nats,
    http::{self, HttpAnswer},
    redis,
    sqlx::PgPool,
};
use tokio::{net::TcpListener, signal};

/// How long `/readyz` fails before the server stops, so load balancers and kube-proxy stop
/// sending traffic first.
const DRAIN_DELAY: Duration = Duration::from_secs(5);

fn setting(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

/// The answer as an axum response.
fn respond(answer: HttpAnswer) -> Response {
    (
        StatusCode::from_u16(answer.status).unwrap_or(StatusCode::SERVICE_UNAVAILABLE),
        [
            (header::CONTENT_TYPE, answer.content_type),
            (header::CACHE_CONTROL, answer.cache_control),
        ],
        answer.body,
    )
        .into_response()
}

async fn livez(State(registry): State<HealthRegistry>, RawQuery(query): RawQuery) -> Response {
    respond(http::probe(&registry, Probe::Live, query.as_deref()))
}

async fn readyz(State(registry): State<HealthRegistry>, RawQuery(query): RawQuery) -> Response {
    respond(http::probe(&registry, Probe::Ready, query.as_deref()))
}

async fn health(State(registry): State<HealthRegistry>) -> Response {
    respond(http::report(&registry))
}

/// `/livez` and `/readyz` for Kubernetes probes, and `/health` for people and dashboards.
fn health_routes(registry: HealthRegistry) -> Router {
    Router::new()
        .route(http::LIVE_PATH, get(livez))
        .route(http::READY_PATH, get(readyz))
        .route(http::REPORT_PATH, get(health))
        .with_state(registry)
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
    let admin = health_routes(registry.clone());
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

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use sisa_health_checks::HealthResult;
    use tower::ServiceExt;

    use super::*;

    async fn get_path(registry: HealthRegistry, path: &str) -> Response {
        let request = Request::get(path)
            .body(Body::empty())
            .expect("the test builds a valid request");

        health_routes(registry)
            .oneshot(request)
            .await
            .expect("the router never fails")
    }

    #[tokio::test]
    async fn a_route_sends_the_whole_answer() {
        let registry = HealthRegistry::builder()
            .check_fn("nats", async || HealthResult::unhealthy("disconnected"))
            .build()
            .expect("the test builds a valid registry");

        registry.run().await;

        let answer = http::probe(&registry, Probe::Ready, Some("verbose"));

        let response = get_path(registry, "/readyz?verbose").await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            answer.content_type
        );
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            answer.cache_control
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is in memory");

        assert_eq!(body, answer.body.as_bytes());
    }

    #[tokio::test]
    async fn there_is_no_healthz() {
        let registry = HealthRegistry::builder()
            .build()
            .expect("the test builds a valid registry");

        let response = get_path(registry, "/healthz").await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

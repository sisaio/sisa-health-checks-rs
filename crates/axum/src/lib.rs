//! The health routes of Sisa Health Checks for [axum](https://docs.rs/axum): `/livez` and
//! `/readyz` for Kubernetes probes, and `/health` for people and dashboards.
//!
//! | Route | Answers | Body |
//! | --- | --- | --- |
//! | `GET /livez` | 200 unless a check tagged `live` fails | `ok`, or the failing lines |
//! | `GET /readyz` | 200 unless a check tagged `ready` fails, the registry drains, or it isn't started | `ok`, or the failing lines |
//! | `GET /livez?verbose`, `GET /readyz?verbose` | The same status | Every line, as `[+]postgres ok` |
//! | `GET /health` | 503 when any check is unhealthy, else 200 | `application/health+json` with every check |
//!
//! The paths and the text answers follow the Kubernetes API server's own `/livez` and `/readyz`.
//! There is no `/healthz`: Kubernetes deprecated it, as one endpoint can't answer both questions.
//! Point a startup probe at `/livez` with a generous `failureThreshold`.
//!
//! The probe routes say only whether each check passed, so they can stay open to the kubelet. The
//! report shows details, so serve it behind your own authorization, or on an admin port only:
//!
//! ```
//! use axum::Router;
//! use sisa_health_checks_axum::{probe_routes, report_routes};
//! use sisa_health_checks_core::HealthRegistry;
//!
//! # fn example(registry: HealthRegistry) {
//! let admin: Router = Router::new()
//!     .merge(probe_routes(registry.clone()))
//!     .merge(report_routes(registry));
//! # let _ = admin;
//! # }
//! ```
//!
//! Every answer has `Cache-Control: no-store`, so no proxy serves a stale one.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

use axum::{
    Router,
    extract::{RawQuery, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sisa_health_checks_core::{HealthRegistry, Probe};

/// The liveness route.
pub const LIVE_PATH: &str = "/livez";

/// The readiness route.
pub const READY_PATH: &str = "/readyz";

/// The report route.
pub const REPORT_PATH: &str = "/health";

const NO_STORE: &str = "no-store";
const TEXT: &str = "text/plain; charset=utf-8";
const HEALTH_JSON: &str = "application/health+json";

/// Whether the query asks for every line: `?verbose`, or `?verbose=` with any value.
fn is_verbose(query: Option<&str>) -> bool {
    query.is_some_and(|query| {
        query
            .split('&')
            .any(|pair| pair == "verbose" || pair.starts_with("verbose="))
    })
}

/// The answer to `probe`: 200 or 503, with the text the Kubernetes API server would write.
fn probe_response(registry: &HealthRegistry, probe: Probe, query: Option<&str>) -> Response {
    let report = registry.probe(probe);
    let status = if report.passed() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        [
            (header::CONTENT_TYPE, TEXT),
            (header::CACHE_CONTROL, NO_STORE),
        ],
        report.to_text(is_verbose(query)),
    )
        .into_response()
}

async fn livez(State(registry): State<HealthRegistry>, RawQuery(query): RawQuery) -> Response {
    probe_response(&registry, Probe::Live, query.as_deref())
}

async fn readyz(State(registry): State<HealthRegistry>, RawQuery(query): RawQuery) -> Response {
    probe_response(&registry, Probe::Ready, query.as_deref())
}

async fn health(State(registry): State<HealthRegistry>) -> Response {
    let report = registry.report();
    let status =
        StatusCode::from_u16(report.http_status_code()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE);

    (
        status,
        [
            (header::CONTENT_TYPE, HEALTH_JSON),
            (header::CACHE_CONTROL, NO_STORE),
        ],
        report.to_json().to_string(),
    )
        .into_response()
}

/// `GET /livez` and `GET /readyz`, for Kubernetes probes, as a router to merge into any other.
pub fn probe_routes<S>(registry: HealthRegistry) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(LIVE_PATH, get(livez))
        .route(READY_PATH, get(readyz))
        .with_state(registry)
}

/// `GET /health`, the full report, as a router to merge into any other. It shows each check's
/// description and details, so put it behind your authorization layer or on an admin port.
pub fn report_routes<S>(registry: HealthRegistry) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(REPORT_PATH, get(health))
        .with_state(registry)
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use sisa_health_checks_core::{HealthResult, LIVE};
    use tower::ServiceExt;

    use super::*;

    async fn get_path(router: Router, path: &str) -> (StatusCode, Option<String>, String) {
        let request = Request::get(path)
            .body(Body::empty())
            .expect("the test builds a valid request");
        let response = router
            .oneshot(request)
            .await
            .expect("the router never fails");
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is in memory");

        (
            status,
            content_type,
            String::from_utf8(body.to_vec()).expect("the body is UTF-8"),
        )
    }

    async fn ran(builder: sisa_health_checks_core::HealthRegistryBuilder) -> HealthRegistry {
        let registry = builder.build().expect("the test builds a valid registry");

        registry.run().await;
        registry
    }

    fn routes(registry: HealthRegistry) -> Router {
        Router::new()
            .merge(probe_routes(registry.clone()))
            .merge(report_routes(registry))
    }

    #[test]
    fn verbose_is_read_from_the_query() {
        assert!(is_verbose(Some("verbose")));
        assert!(is_verbose(Some("exclude=x&verbose=1")));
        assert!(!is_verbose(Some("verbosely")));
        assert!(!is_verbose(None));
    }

    #[tokio::test]
    async fn a_passing_probe_answers_200_ok_in_plain_text() {
        let registry = ran(HealthRegistry::builder()
            .check_fn("event-loop", async || HealthResult::healthy())
            .tags([LIVE]))
        .await;

        let (status, content_type, body) = get_path(routes(registry), "/livez").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/plain; charset=utf-8"));
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn a_failing_probe_answers_503_with_its_failures() {
        let registry = ran(HealthRegistry::builder()
            .check_fn("nats", async || HealthResult::unhealthy("disconnected")))
        .await;

        let (status, _, body) = get_path(routes(registry), "/readyz").await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body, "[-]nats failed: disconnected\nreadyz check failed\n");
    }

    #[tokio::test]
    async fn verbose_lists_every_line() {
        let registry =
            ran(HealthRegistry::builder().check_fn("postgres", async || HealthResult::healthy()))
                .await;

        let (status, _, body) = get_path(routes(registry), "/readyz?verbose").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            "[+]postgres ok\n[+]shutdown ok\n[+]poller ok\nreadyz check passed\n"
        );
    }

    #[tokio::test]
    async fn the_report_is_health_json() {
        let registry = ran(HealthRegistry::builder()
            .check_fn("postgres", async || HealthResult::healthy())
            .check_fn("search", async || HealthResult::degraded("rebuilding")))
        .await;

        let (status, content_type, body) = get_path(routes(registry), "/health").await;
        let report: serde_json::Value = serde_json::from_str(&body).expect("the report is JSON");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("application/health+json"));
        assert_eq!(report["status"], "warn");
        assert_eq!(report["checks"]["search"][0]["output"], "rebuilding");
    }

    #[tokio::test]
    async fn there_is_no_healthz() {
        let registry = ran(HealthRegistry::builder()).await;

        let (status, _, _) = get_path(routes(registry), "/healthz").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}

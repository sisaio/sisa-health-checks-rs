//! The HTTP answers of Sisa Health Checks, in plain data, for any web framework: `/livez` and
//! `/readyz` for Kubernetes probes, and `/health` for people and dashboards.
//!
//! | Request | Call | Answers | Body |
//! | --- | --- | --- | --- |
//! | `GET /livez` | [`probe`] with `Probe::Live` | 200 unless a check tagged `live` fails | `ok`, or the failing lines |
//! | `GET /readyz` | [`probe`] with `Probe::Ready` | 200 unless a check tagged `ready` fails, the registry drains, or it isn't started | `ok`, or the failing lines |
//! | `GET /livez?verbose`, `GET /readyz?verbose` | [`probe`] with the query | The same status | Every line, as `[+]postgres ok` |
//! | `GET /health` | [`report`] | 503 when any check is unhealthy or the results are stale, else 200 | `application/health+json` with every check |
//!
//! Each answer is a status code, a content type, a `Cache-Control` value and a body. Web
//! frameworks don't share one response type, so the answer is none of theirs: a handler copies
//! the four fields into its own. Routing stays with the framework, on [`LIVE_PATH`],
//! [`READY_PATH`] and [`REPORT_PATH`]. In [axum](https://docs.rs/axum):
//!
//! ```
//! use axum::{
//!     Router,
//!     extract::{RawQuery, State},
//!     http::{StatusCode, header},
//!     response::{IntoResponse, Response},
//!     routing::get,
//! };
//! use sisa_health_checks_core::{HealthRegistry, Probe};
//! use sisa_health_checks_http::{self as http, HttpAnswer};
//!
//! fn respond(answer: HttpAnswer) -> Response {
//!     (
//!         StatusCode::from_u16(answer.status).unwrap_or(StatusCode::SERVICE_UNAVAILABLE),
//!         [
//!             (header::CONTENT_TYPE, answer.content_type),
//!             (header::CACHE_CONTROL, answer.cache_control),
//!         ],
//!         answer.body,
//!     )
//!         .into_response()
//! }
//!
//! async fn livez(State(registry): State<HealthRegistry>, RawQuery(query): RawQuery) -> Response {
//!     respond(http::probe(&registry, Probe::Live, query.as_deref()))
//! }
//!
//! async fn readyz(State(registry): State<HealthRegistry>, RawQuery(query): RawQuery) -> Response {
//!     respond(http::probe(&registry, Probe::Ready, query.as_deref()))
//! }
//!
//! async fn health(State(registry): State<HealthRegistry>) -> Response {
//!     respond(http::report(&registry))
//! }
//!
//! fn health_routes(registry: HealthRegistry) -> Router {
//!     Router::new()
//!         .route(http::LIVE_PATH, get(livez))
//!         .route(http::READY_PATH, get(readyz))
//!         .route(http::REPORT_PATH, get(health))
//!         .with_state(registry)
//! }
//! # let _ = health_routes;
//! ```
//!
//! Other frameworks differ only in `respond` and in how a handler gets the registry and the query.
//!
//! The paths and the text answers follow the Kubernetes API server's own `/livez` and `/readyz`.
//! There is no `/healthz`: Kubernetes deprecated it, as one endpoint can't answer both questions.
//! Point a startup probe at `/livez` with a generous `failureThreshold`.
//!
//! The probe answers show each check's name, whether it passed and, when it failed or is degraded,
//! its description, which a check writes to be safe to show, so they can stay open to the kubelet.
//! The report shows details, so serve it behind your own authorization, or on an admin port only.
//!
//! Every answer has `Cache-Control: no-store`, so no proxy serves a stale one.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

use sisa_health_checks_core::{HealthRegistry, Probe};

/// The liveness path.
pub const LIVE_PATH: &str = "/livez";

/// The readiness path.
pub const READY_PATH: &str = "/readyz";

/// The report path.
pub const REPORT_PATH: &str = "/health";

const NO_STORE: &str = "no-store";
const TEXT: &str = "text/plain; charset=utf-8";
const HEALTH_JSON: &str = "application/health+json";

/// What to send back for one health request. Copy each field into the framework's own response.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpAnswer {
    /// The status code: 200 or 503.
    pub status: u16,
    /// The `Content-Type` header.
    pub content_type: &'static str,
    /// The `Cache-Control` header: `no-store`, so no proxy serves a stale answer.
    pub cache_control: &'static str,
    /// The body.
    pub body: String,
}

/// Whether the query asks for every line: `?verbose`, or `?verbose=` with any value.
fn is_verbose(query: Option<&str>) -> bool {
    query.is_some_and(|query| {
        query
            .split('&')
            .any(|pair| pair == "verbose" || pair.starts_with("verbose="))
    })
}

/// The answer to `GET /livez` or `GET /readyz`: 200 or 503, with the text the Kubernetes API
/// server would write. `query` is the request's query string, without the `?`: with `verbose`
/// in it, the text lists every line.
pub fn probe(registry: &HealthRegistry, probe: Probe, query: Option<&str>) -> HttpAnswer {
    let report = registry.probe(probe);

    HttpAnswer {
        status: if report.passed() { 200 } else { 503 },
        content_type: TEXT,
        cache_control: NO_STORE,
        body: report.to_text(is_verbose(query)),
    }
}

/// The answer to `GET /health`: every check's latest result in `application/health+json`, 503
/// when a check is unhealthy or the results are stale, else 200.
pub fn report(registry: &HealthRegistry) -> HttpAnswer {
    let report = registry.report();

    HttpAnswer {
        status: report.http_status_code(),
        content_type: HEALTH_JSON,
        cache_control: NO_STORE,
        body: report.to_json().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use sisa_health_checks_core::{HealthRegistryBuilder, HealthResult, LIVE};

    use super::*;

    async fn ran(builder: HealthRegistryBuilder) -> HealthRegistry {
        let registry = builder.build().expect("the test builds a valid registry");

        registry.run().await;
        registry
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

        assert_eq!(
            probe(&registry, Probe::Live, None),
            HttpAnswer {
                status: 200,
                content_type: "text/plain; charset=utf-8",
                cache_control: "no-store",
                body: "ok".into(),
            }
        );
    }

    #[tokio::test]
    async fn a_failing_probe_answers_503_with_its_failures() {
        let registry = ran(HealthRegistry::builder()
            .check_fn("nats", async || HealthResult::unhealthy("disconnected")))
        .await;

        assert_eq!(
            probe(&registry, Probe::Ready, None),
            HttpAnswer {
                status: 503,
                content_type: "text/plain; charset=utf-8",
                cache_control: "no-store",
                body: "[-]nats failed: disconnected\nreadyz check failed\n".into(),
            }
        );
    }

    #[tokio::test]
    async fn verbose_lists_every_line() {
        let registry =
            ran(HealthRegistry::builder().check_fn("postgres", async || HealthResult::healthy()))
                .await;

        let answer = probe(&registry, Probe::Ready, Some("verbose"));

        assert_eq!(answer.status, 200);
        assert_eq!(
            answer.body,
            "[+]postgres ok\n[+]shutdown ok\n[+]poller ok\nreadyz check passed\n"
        );
    }

    #[tokio::test]
    async fn the_report_answers_in_health_json() {
        let registry = ran(HealthRegistry::builder()
            .check_fn("postgres", async || HealthResult::healthy())
            .check_fn("search", async || HealthResult::degraded("rebuilding")))
        .await;

        let answer = report(&registry);
        let body: serde_json::Value =
            serde_json::from_str(&answer.body).expect("the report is JSON");

        assert_eq!(answer.status, 200);
        assert_eq!(answer.content_type, "application/health+json");
        assert_eq!(answer.cache_control, "no-store");
        assert_eq!(body["status"], "warn");
        assert_eq!(body["checks"]["search"][0]["output"], "rebuilding");
    }

    #[test]
    fn there_is_no_healthz() {
        assert_eq!(
            [LIVE_PATH, READY_PATH, REPORT_PATH],
            ["/livez", "/readyz", "/health"]
        );
    }
}

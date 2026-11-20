//! The probes and the report as HTTP answers, in plain data, for any web framework.
//!
//! Each answer is a status code, a content type, a `Cache-Control` value and a body. Web
//! frameworks don't share one response type, so the answer is none of theirs: a handler copies
//! the four fields into its own. Routing stays with the framework, on [`LIVE_PATH`],
//! [`READY_PATH`] and [`REPORT_PATH`].
//!
//! | Path | Call | Answers |
//! | --- | --- | --- |
//! | `GET /livez` | [`probe`] with [`Probe::Live`] | 200 or 503, in plain text |
//! | `GET /readyz` | [`probe`] with [`Probe::Ready`] | 200 or 503, in plain text |
//! | `GET /health` | [`report`] | 200 or 503, in `application/health+json` |
//!
//! ```
//! use sisa_health_checks_core::{HealthRegistry, HealthResult, Probe, http};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), sisa_health_checks_core::HealthError> {
//! let registry = HealthRegistry::builder()
//!     .check_fn("postgres", async || HealthResult::unhealthy("SELECT 1 failed"))
//!     .build()?;
//! # registry.run().await;
//!
//! // In a handler for `GET /readyz`, with the request's query string, if it has one.
//! let answer = http::probe(&registry, Probe::Ready, Some("verbose"));
//!
//! assert_eq!(answer.status, 503);
//! assert_eq!(answer.content_type, "text/plain; charset=utf-8");
//! assert_eq!(answer.cache_control, "no-store");
//! assert!(answer.body.starts_with("[-]postgres failed: SELECT 1 failed\n"));
//! # Ok(())
//! # }
//! ```
//!
//! The `sisa-health-checks-axum` crate is this module with axum's router around it. The report
//! shows each check's details, so serve it behind your own authorization, or on an admin port.

use crate::{registry::HealthRegistry, report::Probe};

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
    use super::*;
    use crate::result::HealthResult;

    #[test]
    fn verbose_is_read_from_the_query() {
        assert!(is_verbose(Some("verbose")));
        assert!(is_verbose(Some("exclude=x&verbose=1")));
        assert!(!is_verbose(Some("verbosely")));
        assert!(!is_verbose(None));
    }

    #[tokio::test(start_paused = true)]
    async fn a_failing_probe_answers_503_in_plain_text() {
        let registry = HealthRegistry::builder()
            .check_fn("nats", async || HealthResult::unhealthy("disconnected"))
            .build()
            .expect("the test builds a valid registry");

        registry.run().await;

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

    #[tokio::test(start_paused = true)]
    async fn the_report_answers_in_health_json() {
        let registry = HealthRegistry::builder()
            .build()
            .expect("the test builds a valid registry");

        registry.run().await;

        let answer = report(&registry);
        let body: serde_json::Value =
            serde_json::from_str(&answer.body).expect("the report is JSON");

        assert_eq!(answer.status, 200);
        assert_eq!(answer.content_type, "application/health+json");
        assert_eq!(answer.cache_control, "no-store");
        assert_eq!(body, serde_json::json!({ "status": "pass", "checks": {} }));
    }
}

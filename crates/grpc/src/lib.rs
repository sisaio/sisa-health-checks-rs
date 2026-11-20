//! The standard gRPC health service, `grpc.health.v1.Health`, for Sisa Health Checks, on a
//! [connectrpc](https://docs.rs/connectrpc) router. One router serves gRPC, gRPC-Web and Connect
//! clients, so a kubelet `grpc:` probe, Envoy, `grpc_health_probe` and a Connect client all get
//! the same answer.
//!
//! `Check` answers from the registry's latest results, as `/livez` and `/readyz` do, and never
//! runs a check. The request's service name picks the probe:
//!
//! | `service` | Answers | `SERVING` when |
//! | --- | --- | --- |
//! | `""` or `readyz` | readiness | every check tagged `ready` passes, started, not draining |
//! | `livez` | liveness | every check tagged `live` passes |
//! | any other name | | never: the call fails with `NOT_FOUND` |
//!
//! Otherwise the answer is `NOT_SERVING`. Both are `NOT_SERVING` once the results are stale. The
//! answer carries no check name or description, so the service can stay open to the kubelet.
//!
//! ```
//! use connectrpc::Router;
//! use sisa_health_checks_core::HealthRegistry;
//! use sisa_health_checks_grpc::grpc_routes;
//!
//! # fn example(registry: HealthRegistry, services: Router) {
//! let router: Router = services.merge(grpc_routes(registry));
//! # let _ = router;
//! # }
//! ```
//!
//! In Kubernetes, name the service in the probe:
//!
//! ```yaml
//! livenessProbe:
//!   grpc: { port: 8080, service: livez }
//! readinessProbe:
//!   grpc: { port: 8080, service: readyz }
//! ```
//!
//! `Watch` isn't served: it answers `UNIMPLEMENTED`, which the protocol allows. The kubelet,
//! Envoy and `grpc_health_probe` only call `Check`.

use std::sync::Arc;

use connectrpc::{ConnectError, Router};
use connectrpc_health::{
    Checker, HealthExt, HealthService, Status, apply_request_limits, request_limits,
};
use sisa_health_checks_core::{HealthRegistry, Probe};

/// The service name that asks for liveness, as `/livez` does.
pub const LIVE_SERVICE: &str = "livez";

/// The service name that asks for readiness, as `/readyz` does. The empty name asks the same.
pub const READY_SERVICE: &str = "readyz";

/// Answers the health protocol from a registry's latest results.
struct RegistryChecker {
    registry: HealthRegistry,
}

// Picks the probe a service name asks for, and says whether it passes. `watch` keeps the
// trait's own answer, `UNIMPLEMENTED`.
impl Checker for RegistryChecker {
    async fn check(&self, service: &str) -> Result<Status, ConnectError> {
        let probe = match service {
            "" | READY_SERVICE => Probe::Ready,
            LIVE_SERVICE => Probe::Live,
            _ => {
                return Err(ConnectError::not_found(
                    "unknown service: ask for \"livez\", \"readyz\" or the empty name",
                ));
            }
        };

        if self.registry.probe(probe).passed() {
            Ok(Status::Serving)
        } else {
            Ok(Status::NotServing)
        }
    }
}

/// `grpc.health.v1.Health`, answering from `registry`, as a router to merge into any other.
///
/// The two routes take requests of at most 16 KiB, as `connectrpc-health` sets them: a request
/// holds one service name.
pub fn grpc_routes(registry: HealthRegistry) -> Router {
    let service = Arc::new(HealthService::new(RegistryChecker { registry }));

    apply_request_limits(service.register(Router::new()), request_limits())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, Version, header},
    };
    use serde_json::{Value, json};
    use sisa_health_checks_core::{HealthResult, LIVE};
    use tower::ServiceExt;

    use super::*;

    const CHECK_PATH: &str = "/grpc.health.v1.Health/Check";

    /// A registry that has run once: liveness passes, readiness passes when `ready` is true.
    async fn ran(ready: bool) -> HealthRegistry {
        let registry = HealthRegistry::builder()
            .check_fn("event-loop", async || HealthResult::healthy())
            .tags([LIVE])
            // Not an async closure: one that captures `ready` isn't `Fn`.
            .check_fn("postgres", move || async move {
                if ready {
                    HealthResult::healthy()
                } else {
                    HealthResult::unhealthy("SELECT 1 failed")
                }
            })
            .build()
            .expect("the test builds a valid registry");

        registry.run().await;
        registry
    }

    /// Sends one request to the health routes and returns the status code and the body.
    async fn send(registry: HealthRegistry, request: Request<Body>) -> (StatusCode, Vec<u8>) {
        let response = grpc_routes(registry)
            .into_axum_router()
            .oneshot(request)
            .await
            .expect("the router never fails");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is in memory");

        (status, body.to_vec())
    }

    /// Asks `Check` for `service` as a Connect client does, in JSON.
    async fn check(registry: HealthRegistry, service: &str) -> (StatusCode, Value) {
        let request = Request::post(CHECK_PATH)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "service": service }).to_string()))
            .expect("the test builds a valid request");
        let (status, body) = send(registry, request).await;

        (
            status,
            serde_json::from_slice(&body).expect("the answer is JSON"),
        )
    }

    #[tokio::test]
    async fn a_ready_registry_is_serving() {
        let registry = ran(true).await;

        for service in ["", READY_SERVICE, LIVE_SERVICE] {
            let (status, answer) = check(registry.clone(), service).await;

            assert_eq!(status, StatusCode::OK);
            assert_eq!(answer["status"], "SERVING");
        }
    }

    #[tokio::test]
    async fn a_failing_ready_check_stops_readiness_not_liveness() {
        let registry = ran(false).await;

        assert_eq!(check(registry.clone(), "").await.1["status"], "NOT_SERVING");
        assert_eq!(
            check(registry.clone(), READY_SERVICE).await.1["status"],
            "NOT_SERVING"
        );
        assert_eq!(check(registry, LIVE_SERVICE).await.1["status"], "SERVING");
    }

    #[tokio::test]
    async fn an_unknown_service_is_not_found() {
        let (status, answer) = check(ran(true).await, "postgres").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(answer["code"], "not_found");
    }

    #[tokio::test]
    async fn a_grpc_client_gets_the_same_answer() {
        // A gRPC message is one flag byte, a four-byte length, then the protobuf: field 1, the
        // service name, in the request; field 1, the status, in the answer, where 2 is
        // `NOT_SERVING`.
        let request = Request::post(CHECK_PATH)
            .version(Version::HTTP_2)
            .header(header::CONTENT_TYPE, "application/grpc")
            .header(header::TE, "trailers")
            .body(Body::from(b"\0\0\0\0\x08\x0a\x06readyz".to_vec()))
            .expect("the test builds a valid request");
        let (status, body) = send(ran(false).await, request).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"\0\0\0\0\x02\x08\x02");
    }
}

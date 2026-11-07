//! The NATS check against a real server in Docker.

use std::time::Duration;

use sisa_health_checks_core::{HealthRegistry, Probe};
use sisa_health_checks_nats::{NatsCheck, async_nats};
use sisa_health_checks_testing::{Server, eventually};

async fn connect(server: &Server) -> async_nats::Client {
    let url = format!("nats://127.0.0.1:{}", server.port());

    eventually(async || async_nats::connect(url.as_str()).await).await
}

fn registry(client: async_nats::Client) -> HealthRegistry {
    HealthRegistry::builder()
        .check("nats", NatsCheck::new(client))
        .timeout(Duration::from_secs(2))
        .build()
        .expect("the test builds a valid registry")
}

#[tokio::test]
async fn a_running_server_is_healthy() {
    let server = Server::nats().await;
    let health = registry(connect(&server).await);

    health.run().await;

    assert!(health.probe(Probe::Ready).passed());
}

#[tokio::test]
async fn a_stopped_server_fails_readiness() {
    let server = Server::nats().await;
    let health = registry(connect(&server).await);

    server.stop().await;
    // Give the client a moment to see the connection close.
    tokio::time::sleep(Duration::from_secs(1)).await;
    health.run().await;

    assert!(!health.probe(Probe::Ready).passed());
}

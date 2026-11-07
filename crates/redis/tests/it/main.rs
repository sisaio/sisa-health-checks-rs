//! The Redis check against a real server in Docker.

use std::time::Duration;

use sisa_health_checks_core::{HealthRegistry, Probe};
use sisa_health_checks_redis::{
    RedisCheck,
    redis::{Client, aio::ConnectionManager},
};
use sisa_health_checks_testing::{Server, eventually};

async fn connect(server: &Server) -> ConnectionManager {
    let client =
        Client::open(format!("redis://127.0.0.1:{}", server.port())).expect("the URL is valid");

    eventually(async || client.get_connection_manager().await).await
}

fn registry(connection: ConnectionManager) -> HealthRegistry {
    HealthRegistry::builder()
        .check("redis", RedisCheck::new(connection))
        .timeout(Duration::from_secs(2))
        .build()
        .expect("the test builds a valid registry")
}

#[tokio::test]
async fn a_running_server_is_healthy() {
    let server = Server::redis().await;
    let health = registry(connect(&server).await);

    health.run().await;

    assert!(health.probe(Probe::Ready).passed());
}

#[tokio::test]
async fn a_stopped_server_fails_readiness() {
    let server = Server::redis().await;
    let health = registry(connect(&server).await);

    server.stop().await;
    health.run().await;

    assert!(!health.probe(Probe::Ready).passed());
}

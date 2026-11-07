//! The Postgres check against a real server in Docker.

use std::time::Duration;

use sisa_health_checks_core::{HealthRegistry, Probe};
use sisa_health_checks_postgres::{
    PostgresCheck,
    sqlx::{PgPool, postgres::PgPoolOptions},
};
use sisa_health_checks_testing::{Server, eventually};

async fn connect(server: &Server) -> PgPool {
    let url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        server.port()
    );

    eventually(async || {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(1))
            .connect(&url)
            .await
    })
    .await
}

fn registry(pool: PgPool) -> HealthRegistry {
    HealthRegistry::builder()
        .check("postgres", PostgresCheck::new(pool))
        .timeout(Duration::from_secs(3))
        .build()
        .expect("the test builds a valid registry")
}

#[tokio::test]
async fn a_running_server_is_healthy() {
    let server = Server::postgres().await;
    let health = registry(connect(&server).await);

    health.run().await;

    assert!(health.probe(Probe::Ready).passed());
    assert!(
        health.report().checks()[0]
            .details()
            .contains_key("pool_size")
    );
}

#[tokio::test]
async fn a_stopped_server_fails_readiness() {
    let server = Server::postgres().await;
    let health = registry(connect(&server).await);

    server.stop().await;
    health.run().await;

    assert!(!health.probe(Probe::Ready).passed());
}

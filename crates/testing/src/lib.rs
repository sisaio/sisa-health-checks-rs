//! Test helpers for the Sisa Health Checks crates: a Postgres, Redis or NATS server of its own for
//! each test, in Docker, and a way to wait until it answers.
//!
//! The images can be swapped with `POSTGRES_IMAGE`, `REDIS_IMAGE` and `NATS_IMAGE`, as
//! `name:tag`.

use std::{fmt::Debug, time::Duration};

use testcontainers::{
    ContainerAsync, ContainerRequest, GenericImage, ImageExt, core::IntoContainerPort,
    runners::AsyncRunner,
};

/// How many times [`eventually`] tries, and how long it waits between tries.
const ATTEMPTS: u32 = 120;
const PAUSE: Duration = Duration::from_millis(500);

/// The image `variable` names as `name:tag`, or `default`.
fn image(variable: &str, default: &str) -> GenericImage {
    let named = std::env::var(variable).unwrap_or_else(|_| default.to_string());
    let (name, tag) = named.split_once(':').unwrap_or((named.as_str(), "latest"));

    GenericImage::new(name, tag)
}

/// A server of its own for one test, in Docker. It is removed when this value drops.
pub struct Server {
    container: ContainerAsync<GenericImage>,
    port: u16,
}

impl Server {
    async fn start(image: GenericImage, port: u16, env: &[(&str, &str)]) -> Self {
        let mut request: ContainerRequest<GenericImage> =
            image.with_exposed_port(port.tcp()).into();

        for (name, value) in env {
            request = request.with_env_var(*name, *value);
        }

        let container = request
            .start()
            .await
            .expect("Docker must be running to start a server for the tests");
        let host_port = container
            .get_host_port_ipv4(port)
            .await
            .expect("the container exposes its port");

        Self {
            container,
            port: host_port,
        }
    }

    /// A Postgres server, user and password `postgres`. `POSTGRES_IMAGE` picks the image;
    /// the default is `postgres:17-alpine`.
    pub async fn postgres() -> Self {
        Self::start(
            image("POSTGRES_IMAGE", "postgres:17-alpine"),
            5432,
            &[("POSTGRES_PASSWORD", "postgres")],
        )
        .await
    }

    /// A Redis server. `REDIS_IMAGE` picks the image; the default is `redis:8-alpine`.
    pub async fn redis() -> Self {
        Self::start(image("REDIS_IMAGE", "redis:8-alpine"), 6379, &[]).await
    }

    /// A NATS server. `NATS_IMAGE` picks the image; the default is `nats:2-alpine`.
    pub async fn nats() -> Self {
        Self::start(image("NATS_IMAGE", "nats:2-alpine"), 4222, &[]).await
    }

    /// The host port the server listens on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Kills the server, as a crash would: connections close and new ones are refused.
    pub async fn stop(&self) {
        self.container
            .stop_with_timeout(Some(0))
            .await
            .expect("the container stops");
    }
}

/// Calls `attempt` until it succeeds, for a server that is still starting; panics with the last
/// error after a minute.
pub async fn eventually<T, E, F>(mut attempt: F) -> T
where
    E: Debug,
    F: AsyncFnMut() -> Result<T, E>,
{
    let mut last = None;

    for _ in 0..ATTEMPTS {
        match attempt().await {
            Ok(value) => return value,
            Err(error) => last = Some(error),
        }

        tokio::time::sleep(PAUSE).await;
    }

    panic!("the server never answered: {last:?}");
}

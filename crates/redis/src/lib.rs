//! A Redis check for Sisa Health Checks: `PING` through the application's own
//! [redis-rs](https://docs.rs/redis) connection, which works on Redis, Valkey, Dragonfly and
//! Garnet, and on a cluster connection too.
//!
//! ```no_run
//! use sisa_health_checks_core::{HealthRegistry, Status};
//! use sisa_health_checks_redis::{RedisCheck, redis};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = redis::Client::open("redis://127.0.0.1:6379")?;
//! let connection = client.get_connection_manager().await?;
//! let registry = HealthRegistry::builder()
//!     .check("redis", RedisCheck::new(connection))
//!     // Only a cache here, so the pod can serve without it.
//!     .failure(Status::Degraded)
//!     .build()?;
//! # let _ = registry;
//! # Ok(())
//! # }
//! ```
//!
//! `PING` proves the server answers, not that it accepts writes. For that, write your own check
//! that sets a key of its own with a short TTL and reads it back.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

use std::{fmt, future::Future};

pub use redis;
use redis::{RedisResult, aio::ConnectionLike};
use sisa_health_checks_core::{CheckContext, HealthCheck, HealthResult};

/// Checks a Redis-family server with `PING` through any async redis-rs connection, such as a
/// `ConnectionManager` or a cluster connection.
#[derive(Clone)]
pub struct RedisCheck<C> {
    connection: C,
}

impl<C> RedisCheck<C> {
    /// A check through `connection`. Pass a clone of the application's own, so the check uses
    /// the same connection its requests do.
    pub fn new(connection: C) -> Self {
        Self { connection }
    }
}

// Names the type only: a connection has nothing worth showing.
impl<C> fmt::Debug for RedisCheck<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("RedisCheck").finish_non_exhaustive()
    }
}

// Sends PING and expects PONG.
impl<C> HealthCheck for RedisCheck<C>
where
    C: ConnectionLike + Clone + Send + Sync + 'static,
{
    fn check(&self, _context: &CheckContext) -> impl Future<Output = HealthResult> + Send {
        let mut connection = self.connection.clone();

        async move {
            let ping = redis::cmd("PING");
            let reply: RedisResult<String> = ping.query_async(&mut connection).await;

            match reply {
                Ok(pong) if pong == "PONG" => HealthResult::healthy(),
                Ok(other) => HealthResult::unhealthy(format!("PING answered {other:?}")),
                Err(error) => HealthResult::unhealthy("PING failed").with_error(&error),
            }
        }
    }
}

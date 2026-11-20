//! A NATS check for Sisa Health Checks, on the application's own
//! [async-nats](https://docs.rs/async-nats) client: the connection must be up, and a flush of the
//! client's pending writes must succeed.
//!
//! ```no_run
//! use sisa_health_checks_core::HealthRegistry;
//! use sisa_health_checks_nats::{NatsCheck, async_nats};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = async_nats::connect("nats://127.0.0.1:4222").await?;
//! let registry = HealthRegistry::builder()
//!     .check("nats", NatsCheck::new(client))
//!     .build()?;
//! # let _ = registry;
//! # Ok(())
//! # }
//! ```
//!
//! The flush shows that the client's connection task is alive and its socket still takes writes,
//! not that the server answered: async-nats has no ping an application can send. A connection
//! that died without closing is found by the client's own pings, and fails this check once the
//! client reports it as disconnected.
//!
//! Tag it `ready` when the pod consumes from NATS. When it only publishes through an outbox, the
//! outbox holds messages while NATS is down, so give it `failure(Status::Degraded)` instead.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

use std::fmt;

pub use async_nats;
use async_nats::{Client, connection::State};
use sisa_health_checks_core::{CheckContext, HealthCheck, HealthResult};

/// Checks a NATS server through an async-nats client: the connection must be up and a flush of
/// its pending writes must succeed.
#[derive(Clone)]
pub struct NatsCheck {
    client: Client,
}

impl NatsCheck {
    /// A check through `client`. A client is cheap to clone, so pass a clone of the
    /// application's own.
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

// Names the type only: the client's state is what the check reports.
impl fmt::Debug for NatsCheck {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("NatsCheck").finish_non_exhaustive()
    }
}

// Reads the connection state, then flushes the client's pending writes.
impl HealthCheck for NatsCheck {
    async fn check(&self, _context: &CheckContext) -> HealthResult {
        let state = self.client.connection_state();

        if state != State::Connected {
            return HealthResult::unhealthy(format!("connection is {state:?}"));
        }

        match self.client.flush().await {
            Ok(()) => HealthResult::healthy(),
            Err(error) => HealthResult::unhealthy("flush failed").with_error(&error),
        }
    }
}

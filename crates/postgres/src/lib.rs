//! A PostgreSQL check for Sisa Health Checks: `SELECT 1` on a connection from the application's
//! own [sqlx](https://docs.rs/sqlx) pool, so it fails when the pool can't reach the server or has
//! no connection to spare.
//!
//! ```no_run
//! use sisa_health_checks_core::HealthRegistry;
//! use sisa_health_checks_postgres::{PostgresCheck, sqlx::PgPool};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = PgPool::connect("postgres://app@localhost/app").await?;
//! let registry = HealthRegistry::builder()
//!     .check("postgres", PostgresCheck::new(pool))
//!     .build()?;
//! # let _ = registry;
//! # Ok(())
//! # }
//! ```
//!
//! The report shows the pool's size and idle connections, so a pool that runs out shows there
//! before requests time out. The check waits for a connection like any query does, so keep the
//! pool's acquire timeout under the check's timeout to see the pool's own error.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

use std::future::Future;

use sisa_health_checks_core::{CheckContext, HealthCheck, HealthResult};
pub use sqlx;
use sqlx::PgPool;

/// Checks a PostgreSQL server through an sqlx pool with `SELECT 1`.
#[derive(Debug, Clone)]
pub struct PostgresCheck {
    pool: PgPool,
}

impl PostgresCheck {
    /// A check on `pool`. The pool is cheap to clone, so pass a clone of the application's own.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// Runs `SELECT 1` and reports the pool's size beside it.
impl HealthCheck for PostgresCheck {
    fn check(&self, _context: &CheckContext) -> impl Future<Output = HealthResult> + Send {
        async move {
            let selected: Result<i32, sqlx::Error> =
                sqlx::query_scalar("SELECT 1").fetch_one(&self.pool).await;
            let result = match selected {
                Ok(_) => HealthResult::healthy(),
                Err(error) => HealthResult::unhealthy("SELECT 1 failed").with_error(&error),
            };

            result
                .with_detail("pool_size", self.pool.size())
                .with_detail("pool_idle", self.pool.num_idle())
        }
    }
}

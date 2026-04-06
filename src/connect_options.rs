pub use mongodb;
use mongodb::{bson::doc, options::ClientOptions, Client, Database};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{debug, error, info};

// Global handle to the `Database` the trait operates against. Set once at
// application startup via `connect()` (or `init()` for advanced use).
static DB: OnceLock<Database> = OnceLock::new();

/// Connection parameters for [`connect`].
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// MongoDB connection URI, e.g. `"mongodb://localhost:27017"`.
    pub uri: String,
    /// Database name to use for all model operations.
    pub db_name: String,
    /// Optional max pool size. Forwarded to `ClientOptions::max_pool_size`.
    pub max_pool_size: Option<u32>,
    /// Optional max idle time for pooled connections. Forwarded to
    /// `ClientOptions::max_idle_time`.
    pub max_idle_time: Option<Duration>,
}

impl ConnectOptions {
    /// Shortcut for a URI + database name with default pool settings.
    pub fn new(uri: impl Into<String>, db_name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            db_name: db_name.into(),
            max_pool_size: None,
            max_idle_time: None,
        }
    }
}

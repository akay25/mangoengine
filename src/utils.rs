pub use mongodb;
use mongodb::{bson::doc, options::ClientOptions, Client, Database};
use std::sync::OnceLock;
use tracing::{debug, error, info};

// Local imports
use crate::connect_options::ConnectOptions;
// Global handle to the `Database` the trait operates against. Set once at
// application startup via `connect()` (or `init()` for advanced use).
static DB: OnceLock<Database> = OnceLock::new();

/// Connect to MongoDB, verify connectivity, and initialize the library so
/// all models can reach the database.
///
/// Call this exactly once at application startup. Subsequent calls are
/// silently ignored (the first-set `Database` handle is kept).
///
/// A `ping` command is issued after connecting to fail fast on unreachable
/// or misconfigured databases.
pub async fn connect(opts: ConnectOptions) -> Result<(), mongodb::error::Error> {
    debug!("Trying to connect to database");

    let mut client_options = ClientOptions::parse(&opts.uri).await?;
    client_options.max_pool_size = opts.max_pool_size;
    client_options.max_idle_time = opts.max_idle_time;

    let client = Client::with_options(client_options)?;
    let database = client.database(&opts.db_name);

    // Fail fast if the database is unreachable.
    database.run_command(doc! {"ping": 1}).await?;

    info!("Connected to mongo database");
    init(database);
    Ok(())
}

/// Low-level initialization: store an already-constructed `Database` handle.
/// Most callers should prefer [`connect`]. Subsequent calls are ignored.
pub fn init(db: Database) {
    let _ = DB.set(db);
}

/// Returns the initialized `Database`. Panics if neither [`connect`] nor
/// [`init`] has been called.
pub fn get_db() -> &'static Database {
    DB.get()
        .expect("mangoengine not initialized — call `mangoengine::connect(...)` at startup")
}

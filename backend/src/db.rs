// ============================================================
// FILE: db.rs
// PURPOSE: PostgreSQL database connection pool setup.
//          Creates and configures the sqlx PgPool used throughout the app.
//          The pool manages multiple database connections efficiently,
//          reusing them across requests instead of opening a new connection
//          for every query.
//
// POOL CONFIGURATION:
//   - max_connections: 20 - maximum simultaneous DB connections
//   - min_connections: 2  - keep at least 2 connections warm (reduces latency)
//   - acquire_timeout: 5s - fail fast if no connection available within 5 seconds
//
// JUNIOR DEV NOTE:
//   A connection pool is like a pool of workers. Instead of hiring a new worker
//   (opening a DB connection) for every task (query), you keep a pool of workers
//   ready and assign tasks to available ones. Much more efficient!
// ============================================================

// Result: anyhow's flexible error type
use anyhow::Result;

// PgPoolOptions: builder for configuring the PostgreSQL connection pool
use sqlx::postgres::PgPoolOptions;

// PgPool: the PostgreSQL connection pool type (cheaply cloneable via Arc internally)
use sqlx::PgPool;

// info!: structured logging macro
use tracing::info;

// ============================================================
// FUNCTION: create_pool
// PURPOSE: Creates and returns a configured PostgreSQL connection pool.
//          Called once at startup in AppState::new().
//          The pool is then stored in AppState and shared across all handlers.
// PARAM database_url: PostgreSQL connection string
//   Format: postgres://username:password@host:port/database_name
//   Example: postgres://polypulse:secret@localhost:5432/polypulse_db
// RETURNS: Result<PgPool> - Ok(pool) if connection succeeds, Err otherwise
// ============================================================
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    info!("Connecting to PostgreSQL");

    // Build and connect the pool with our configuration
    let pool = PgPoolOptions::new()
        // Maximum number of connections in the pool.
        // Higher = more concurrent queries, but more memory and DB server load.
        .max_connections(20)

        // Minimum connections to keep alive even when idle.
        // Keeps connections warm so the first requests don't wait for connection setup.
        .min_connections(2)

        // How long to wait for an available connection before giving up.
        // If all 20 connections are busy and a new request comes in,
        // it waits up to 5 seconds before returning an error.
        .acquire_timeout(std::time::Duration::from_secs(5))

        // Connect to the database using the provided URL.
        // This establishes the initial connections and verifies the URL is valid.
        // The ? propagates any connection error up to the caller.
        .connect(database_url)
        .await?;

    info!("PostgreSQL pool established");

    // Return the configured pool
    Ok(pool)
} // end create_pool

// ============================================================
// FUNCTION: run_migrations
// PURPOSE: Runs any pending database migrations from the migrations/ directory.
//          sqlx::migrate!() is a compile-time macro that embeds migration files
//          into the binary. At runtime, it checks which migrations have been run
//          (tracked in the _sqlx_migrations table) and runs any new ones.
//
// WHEN TO CALL: After creating the pool, before starting the server.
//               Currently called manually - could be added to main.rs startup.
//
// JUNIOR DEV NOTE:
//   Migrations are SQL files that modify the database schema over time.
//   They're numbered (e.g., 20240101_initial.sql) and run in order.
//   Once run, they're never run again (tracked in _sqlx_migrations table).
// ============================================================
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    info!("Running database migrations");

    // sqlx::migrate!("./migrations") embeds all .sql files from the migrations/ directory.
    // .run(pool) executes any migrations that haven't been run yet.
    // The ? propagates any migration error.
    sqlx::migrate!("./migrations").run(pool).await?;

    info!("Migrations complete");
    Ok(())
} // end run_migrations

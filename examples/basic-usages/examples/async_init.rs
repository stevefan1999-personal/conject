#![allow(dead_code)]
//! Example demonstrating async dependency injection with conject.
//!
//! Run with: `cargo run --example async_init -p conject`
//!
//! **Requires `std`** — uses the tokio async runtime (`#[tokio::main]`).

use conject::{async_injectable, injectable, provider};

/// A configuration value that is synchronously injectable.
#[injectable]
#[derive(Debug)]
struct Config {
    #[inject("localhost:5432".to_string())]
    db_url: String,
    #[inject(5)]
    max_retries: i32,
}

/// A database pool that requires async initialization.
#[async_injectable]
#[derive(Debug)]
struct DbPool {
    #[inject("connected-pool".to_string())]
    connection: String,
}

/// A service that depends on both sync and async dependencies.
#[async_injectable]
#[derive(Debug)]
struct AppService {
    config: Config,
    pool: DbPool,
    #[inject("app-v1".to_string())]
    version: String,
}

/// The provider that wires everything together.
#[provider]
struct AppProvider;

#[tokio::main]
async fn main() {
    let provider = AppProvider;

    // Async provide
    let service: AppService = provider.provide_async().await;
    assert_eq!(service.config.db_url, "localhost:5432");
    assert_eq!(service.config.max_retries, 5);
    assert_eq!(service.pool.connection, "connected-pool");
    assert_eq!(service.version, "app-v1");

    // Sync provide still works
    let config: Config = provider.provide();
    assert_eq!(config.db_url, "localhost:5432");
    assert_eq!(config.max_retries, 5);
}

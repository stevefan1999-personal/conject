#![allow(dead_code)]
//! Example demonstrating async dependency injection with nject.
//!
//! Run with: `cargo run --example async_init -p nject`

use nject::{async_injectable, injectable, provider};

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

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    rt.block_on(async {
        let provider = AppProvider;

        // Async provide
        let service: AppService = provider.provide_async().await;
        println!("Service: {service:#?}");

        // Sync provide still works
        let config: Config = provider.provide();
        println!("Config: {config:#?}");
    });
}

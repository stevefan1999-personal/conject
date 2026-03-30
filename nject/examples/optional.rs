#![allow(dead_code)]
//! Example demonstrating optional dependency injection with nject.
//!
//! `Option<T>` fields without `#[inject]` default to `None`.
//! Use `#[inject(Some(...))]` to provide an explicit value.

use nject::{injectable, provider};

/// A required dependency.
#[injectable]
#[derive(Debug)]
struct DatabaseConnection;

/// A struct with both required and optional dependencies.
/// The `cache` field defaults to `None` since no provider supplies it.
/// The `db` field is injected normally.
#[injectable]
#[derive(Debug)]
struct Service {
    db: DatabaseConnection,
    cache: Option<String>,
}

/// A struct where all fields are optional -- no provider constraints needed.
#[injectable]
#[derive(Debug)]
struct OptionalConfig {
    debug_mode: Option<bool>,
    log_level: Option<u8>,
}

/// A struct using `#[inject]` to override the default `None` for an optional field.
#[injectable]
#[derive(Debug)]
struct ServiceWithDefaults {
    db: DatabaseConnection,
    #[inject(Some(String::from("memory")))]
    cache_backend: Option<String>,
    metrics: Option<bool>,
}

#[provider]
struct AppProvider;

fn main() {
    let provider = AppProvider;

    let service: Service = provider.provide();
    println!("Service: {:?}", service);
    // Service { db: DatabaseConnection, cache: None }

    let config: OptionalConfig = provider.provide();
    println!("Config: {:?}", config);
    // OptionalConfig { debug_mode: None, log_level: None }

    let service_with_defaults: ServiceWithDefaults = provider.provide();
    println!("ServiceWithDefaults: {:?}", service_with_defaults);
    // ServiceWithDefaults { db: DatabaseConnection, cache_backend: Some("memory"), metrics: None }
}

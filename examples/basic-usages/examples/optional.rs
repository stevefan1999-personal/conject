#![no_std]
#![allow(dead_code)]
//! Example demonstrating optional dependency injection with conject.
//!
//! `Option<T>` fields without `#[inject]` default to `None`.
//! Use `#[inject(Some(...))]` to provide an explicit value.

// no_std + alloc compatible: uses String and Option from core/alloc.
// `extern crate alloc` provides heap types; `extern crate std` provides the
// binary runtime. Replace std with your own in a real no_std target.
extern crate alloc;
extern crate std;

use alloc::string::String;
use conject::{injectable, provider};

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
    assert!(service.cache.is_none());

    let config: OptionalConfig = provider.provide();
    assert!(config.debug_mode.is_none());
    assert!(config.log_level.is_none());

    let service_with_defaults: ServiceWithDefaults = provider.provide();
    assert_eq!(
        service_with_defaults.cache_backend,
        Some(String::from("memory"))
    );
    assert!(service_with_defaults.metrics.is_none());
}

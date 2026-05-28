#![no_std]
//! typed-builder + conject: external construction, singletons, and efficiency.
//!
//! Demonstrates how to integrate [`typed_builder::TypedBuilder`] with conject's
//! singleton provider patterns. Key ideas:
//!
//! - **External construction**: Build complex types via TypedBuilder *before*
//!   the DI graph, then inject them as pre-built singletons. Useful when
//!   values come from runtime config, environment variables, or CLI flags.
//!
//! - **`AppProvider` is also a TypedBuilder**: all three layers of the stack
//!   (config → pool → provider) share the same fluent builder pattern.
//!   Field-level `#[builder(default = ...)]` gives you a zero-config dev mode.
//!
//! - **`#[singleton]`** stores an owned value `T` in the provider and lends
//!   `&'prov T` to consumers. Zero allocation at `provide()` time.
//!
//! - **`#[provide(Arc<T>, |x| x.clone())]`** stores `Arc<T>` and hands out
//!   cheap refcount clones. All consumers share the same heap allocation.
//!
//! - **Efficiency proof**: `core::ptr::eq` / `Arc::ptr_eq` confirm that every
//!   `provide()` call returns a pointer to the same underlying object.
//!
//! Run with:
//! ```text
//! cargo run --example typed_builder
//! ```

#![allow(dead_code)]

// no_std + alloc compatible: uses Arc, String, and format! from alloc.
// `extern crate alloc` provides heap types; `extern crate std` provides the
// binary runtime. Replace std with your own in a real no_std target.
#[macro_use]
extern crate alloc;
extern crate std;

use alloc::{
    string::{String, ToString},
    sync::Arc,
};
use conject::{injectable, provider};
use typed_builder::TypedBuilder;

// ── Types built with TypedBuilder ─────────────────────────────────────────────

/// Runtime database configuration, typically loaded from env/CLI.
/// TypedBuilder gives ergonomic construction with safe defaults.
#[derive(Debug, Clone, TypedBuilder)]
pub struct DbConfig {
    pub host: String,
    pub database: String,
    /// Defaults to the standard Postgres port.
    #[builder(default = 5432)]
    pub port: u16,
    /// Defaults to a conservative pool size.
    #[builder(default = 10)]
    pub max_connections: u32,
    /// Connection timeout in seconds.
    #[builder(default = 30)]
    pub connect_timeout_secs: u64,
}

/// A database connection pool.
/// Not marked `#[injectable]` — we construct it externally and inject
/// the result as a singleton, so conject never calls its constructor.
#[derive(Debug, TypedBuilder)]
pub struct DbPool {
    pub url: String,
    pub max_connections: u32,
    /// Optional label shown in metrics / logs.
    #[builder(default = "default".to_string())]
    pub label: String,
}

impl DbPool {
    fn from_config(cfg: &DbConfig) -> Self {
        Self::builder()
            .url(format!(
                "postgres://{}:{}/{}",
                cfg.host, cfg.port, cfg.database
            ))
            .max_connections(cfg.max_connections)
            .label("primary".to_string())
            .build()
    }
}

/// Lightweight observability config, shared by reference across services.
#[derive(Debug, TypedBuilder)]
pub struct Metrics {
    #[builder(default = "app".to_string())]
    pub namespace: String,
    #[builder(default = true)]
    pub enabled: bool,
    #[builder(default = 10_000)]
    pub flush_interval_ms: u64,
}

// ── Services that consume the singletons ─────────────────────────────────────

#[injectable]
#[derive(Debug)]
struct UserRepository {
    pool: Arc<DbPool>,
}

#[injectable]
#[derive(Debug)]
struct OrderRepository {
    pool: Arc<DbPool>,
}

/// AppService depends on both repos plus a reference to the Metrics singleton.
/// The lifetime `'a` comes from the `#[singleton] metrics` field on the provider.
#[injectable]
#[derive(Debug)]
struct AppService<'a> {
    users: UserRepository,
    orders: OrderRepository,
    metrics: &'a Metrics,
}

/// A background worker that also needs its own Arc handle.
#[injectable]
#[derive(Debug)]
struct BackgroundWorker {
    pool: Arc<DbPool>,
}

// ── Provider ──────────────────────────────────────────────────────────────────
//
// AppProvider is ALSO a TypedBuilder — the entire stack uses the same pattern.
//
// Field-level defaults create a zero-config dev mode:
//
//   AppProvider::builder().build()          ← dev: local Postgres, defaults
//   AppProvider::builder().pool(p).build()  ← prod: supply your own pool
//
// Two singleton patterns:
//
//  pool:    Arc<DbPool>  ── #[provide(Arc<DbPool>, |x| x.clone())]
//              Each provide() is Arc::clone — O(1), shared heap allocation.
//              Consumers hold Arc<DbPool> (owned, lifetime-free).
//
//  metrics: Metrics      ── #[singleton]
//              Each provide() is &'prov Metrics — zero cost, just a pointer.
//              Consumers hold &'a Metrics (borrowed, needs lifetime param).

#[provider]
#[derive(TypedBuilder)]
struct AppProvider {
    /// All consumers receive `Arc::clone` of this same allocation.
    /// Defaults to a local dev pool so `AppProvider::builder().build()` works.
    #[provide(Arc<DbPool>, |x| x.clone())]
    #[builder(default = Arc::new(
        DbPool::builder()
            .url("postgres://localhost/dev".to_string())
            .max_connections(5)
            .label("dev".to_string())
            .build()
    ))]
    pool: Arc<DbPool>,

    /// All consumers receive `&'prov Metrics` — no clone at all.
    /// Defaults to all-defaults Metrics so dev mode works out of the box.
    #[singleton]
    #[builder(default = Metrics::builder().build())]
    metrics: Metrics,
}

fn main() {
    // ── Dev mode: zero-config, pure defaults ─────────────────────────────────
    let dev = AppProvider::builder().build();

    let dev_pool: Arc<DbPool> = dev.provide();
    assert_eq!(dev_pool.url, "postgres://localhost/dev");
    assert_eq!(dev_pool.label, "dev");
    assert_eq!(dev_pool.max_connections, 5);

    // ── Production: explicit construction via TypedBuilder ───────────────────
    let config = DbConfig::builder()
        .host("prod-db.internal".to_string())
        .database("orders".to_string())
        .max_connections(50)
        // port (5432) and connect_timeout_secs (30) use their defaults
        .build();

    // Three-layer fluent chain: DbConfig → DbPool → AppProvider
    let provider = AppProvider::builder()
        .pool(Arc::new(DbPool::from_config(&config)))
        .metrics(Metrics::builder().namespace("myapp".to_string()).build())
        .build();

    // ── Efficiency: Arc<T> singleton ──────────────────────────────────────────
    let p1: Arc<DbPool> = provider.provide();
    let p2: Arc<DbPool> = provider.provide();
    assert!(Arc::ptr_eq(&p1, &p2));

    // ── Efficiency: &T singleton ──────────────────────────────────────────────
    let m1: &Metrics = provider.provide();
    let m2: &Metrics = provider.provide();
    assert!(core::ptr::eq(m1, m2));

    // ── Full service graph ────────────────────────────────────────────────────
    let svc: AppService = provider.provide();
    let worker: BackgroundWorker = provider.provide();

    // Both repos inside AppService got an Arc clone of the same pool.
    assert!(Arc::ptr_eq(&svc.users.pool, &svc.orders.pool));
    // The worker's pool is also the same allocation.
    assert!(Arc::ptr_eq(&svc.users.pool, &worker.pool));
    // The metrics reference inside AppService is the exact same address.
    assert!(core::ptr::eq(svc.metrics, m1));

    // ── Value assertions ──────────────────────────────────────────────────────
    assert_eq!(
        svc.users.pool.url,
        "postgres://prod-db.internal:5432/orders"
    );
    assert_eq!(svc.users.pool.max_connections, 50);
    assert_eq!(svc.users.pool.label, "primary");
    assert_eq!(svc.metrics.namespace, "myapp");
    assert!(svc.metrics.enabled);
}

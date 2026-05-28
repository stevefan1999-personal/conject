#![no_std]
//! Example: Overriding provider bindings for testing.
//!
//! This demonstrates the recommended pattern for swapping implementations
//! in tests without modifying any production code or macros. The key idea:
//! define a test-specific `#[provider]` that uses `#[provide]` to supply
//! mock/stub implementations for the types you want to override.
#![allow(dead_code)]

// no_std + alloc compatible: uses String, Vec, and Box from alloc.
// `extern crate alloc` provides heap types; `extern crate std` provides the
// binary runtime. Replace std with your own in a real no_std target.
#[macro_use]
extern crate alloc;
extern crate std;

use alloc::{boxed::Box, string::String, vec::Vec};
use conject::{injectable, module, provider};

// ── Production domain types ────────────────────────────────────────

trait Cache: core::fmt::Debug {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str);
}

trait Database: core::fmt::Debug {
    fn query(&self, sql: &str) -> Vec<String>;
}

#[derive(Debug, Clone)]
struct AppConfig {
    db_url: String,
    cache_ttl: u32,
}

// ── Production implementations ─────────────────────────────────────

#[derive(Debug)]
struct RedisCache {
    ttl: u32,
}

impl Cache for RedisCache {
    fn get(&self, key: &str) -> Option<String> {
        // In production this would call Redis
        Some(format!("redis:{}:ttl={}", key, self.ttl))
    }
    fn set(&self, _key: &str, _value: &str) {
        // In production this would write to Redis
    }
}

#[derive(Debug)]
struct PostgresDb {
    url: String,
}

impl Database for PostgresDb {
    fn query(&self, sql: &str) -> Vec<String> {
        // In production this would query Postgres
        vec![format!("pg-result({}): {}", self.url, sql)]
    }
}

// ── Application services ───────────────────────────────────────────

#[injectable]
#[derive(Debug)]
struct UserRepository {
    db: Box<dyn Database>,
    cache: Box<dyn Cache>,
}

impl UserRepository {
    fn find_user(&self, id: i32) -> String {
        if let Some(cached) = self.cache.get(&format!("user:{}", id)) {
            return cached;
        }
        let results = self
            .db
            .query(&format!("SELECT * FROM users WHERE id = {}", id));
        results.into_iter().next().unwrap_or_default()
    }
}

#[injectable]
#[derive(Debug)]
struct UserService {
    repo: UserRepository,
    config: AppConfig,
}

// ── Production module + provider ───────────────────────────────────

#[injectable]
#[module]
#[export(AppConfig, AppConfig { db_url: "postgres://prod:5432/app".into(), cache_ttl: 300 })]
struct ConfigModule;

// Production provider wires everything together
#[injectable]
#[provider]
#[provide(Box<dyn Database>, |config: AppConfig| -> Box<dyn Database> {
    Box::new(PostgresDb { url: config.db_url.clone() })
})]
#[provide(Box<dyn Cache>, |config: AppConfig| -> Box<dyn Cache> {
    Box::new(RedisCache { ttl: config.cache_ttl })
})]
struct AppProvider(#[import] ConfigModule);

// ── Test mocks ─────────────────────────────────────────────────────

#[derive(Debug)]
struct MockCache {
    response: String,
}

impl Cache for MockCache {
    fn get(&self, _key: &str) -> Option<String> {
        Some(self.response.clone())
    }
    fn set(&self, _key: &str, _value: &str) {}
}

#[derive(Debug)]
struct MockDatabase {
    response: String,
}

impl Database for MockDatabase {
    fn query(&self, _sql: &str) -> Vec<String> {
        vec![self.response.clone()]
    }
}

fn main() {
    // Production provider: real Redis cache, real Postgres DB
    {
        #[provider]
        struct InitProvider;
        let provider = InitProvider.provide::<AppProvider>();

        let svc: UserService = provider.provide();
        assert_eq!(svc.config.db_url, "postgres://prod:5432/app");
        assert_eq!(svc.config.cache_ttl, 300);
        // RedisCache returns a cached value immediately for any key
        assert_eq!(svc.repo.find_user(1), "redis:user:1:ttl=300");
    }

    // Test provider: override both Database and Cache with mocks
    {
        #[provider]
        #[provide(AppConfig, AppConfig { db_url: "test://memory".into(), cache_ttl: 0 })]
        #[provide(Box<dyn Database>, Box::new(MockDatabase { response: "mock-user-alice".into() }))]
        #[provide(Box<dyn Cache>, Box::new(MockCache { response: "cached-alice".into() }))]
        struct TestProvider;

        let svc: UserService = TestProvider.provide();
        assert_eq!(svc.config.db_url, "test://memory");
        assert_eq!(svc.config.cache_ttl, 0);
        assert_eq!(svc.repo.find_user(1), "cached-alice");
    }

    // Test provider: override only Cache, keep real Database
    {
        #[provider]
        #[provide(AppConfig, AppConfig { db_url: "postgres://test:5432/testdb".into(), cache_ttl: 0 })]
        #[provide(Box<dyn Database>, |config: AppConfig| -> Box<dyn Database> {
            Box::new(PostgresDb { url: config.db_url.clone() })
        })]
        #[provide(Box<dyn Cache>, Box::new(MockCache { response: "test-cached".into() }))]
        struct PartialTestProvider;

        let svc: UserService = PartialTestProvider.provide();
        assert_eq!(svc.config.db_url, "postgres://test:5432/testdb");
        assert_eq!(svc.repo.find_user(1), "test-cached");
    }
}

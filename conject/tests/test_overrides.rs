//! Tests demonstrating how to use existing conject features to override
//! provider bindings for testing, without modifying any macro code.
//!
//! Pattern: define a test-specific `#[provider]` that uses `#[provide]`
//! to supply mock/stub implementations for the types under test.
#![allow(dead_code)]

use conject::{injectable, module, provider};

// ── Shared production types ────────────────────────────────────────

trait Cache: core::fmt::Debug {
    fn get(&self, key: &str) -> Option<String>;
}

#[derive(Debug)]
struct RedisCache;
impl Cache for RedisCache {
    fn get(&self, _key: &str) -> Option<String> {
        Some("redis-value".into())
    }
}

#[derive(Debug)]
struct MockCache;
impl Cache for MockCache {
    fn get(&self, _key: &str) -> Option<String> {
        Some("mock-value".into())
    }
}

#[injectable]
#[derive(Debug, PartialEq)]
struct UserService {
    #[inject(42)]
    id: i32,
}

// ── Tests ──────────────────────────────────────────────────────────

#[test]
fn override_single_binding_for_tests() {
    // Production provider: provides RedisCache
    #[provider]
    #[provide(Box<dyn Cache>, Box::new(RedisCache))]
    struct AppProvider;

    // Test provider: overrides Cache with MockCache
    #[provider]
    #[provide(Box<dyn Cache>, Box::new(MockCache))]
    struct TestProvider;

    let prod_cache: Box<dyn Cache> = AppProvider.provide();
    assert_eq!(prod_cache.get("key"), Some("redis-value".into()));

    let test_cache: Box<dyn Cache> = TestProvider.provide();
    assert_eq!(test_cache.get("key"), Some("mock-value".into()));
}

#[test]
fn override_preserves_other_bindings() {
    // TestProvider overrides Cache but still provides injectable types unchanged
    #[provider]
    #[provide(Box<dyn Cache>, Box::new(MockCache))]
    struct TestProvider;

    let svc: UserService = TestProvider.provide();
    assert_eq!(svc.id, 42);

    let cache: Box<dyn Cache> = TestProvider.provide();
    assert_eq!(cache.get("k"), Some("mock-value".into()));
}

#[test]
fn override_with_module_exports() {
    #[injectable]
    #[module]
    #[export(i32, 42)]
    struct ConfigModule;

    // Production provider imports the module, getting i32 = 42
    #[injectable]
    #[provider]
    struct ProdProvider(#[import] ConfigModule);

    // Test provider overrides i32 directly
    #[provider]
    #[provide(i32, 999)]
    struct TestProvider;

    #[provider]
    struct InitProvider;

    let prod_val: i32 = InitProvider.provide::<ProdProvider>().provide();
    let test_val: i32 = TestProvider.provide();

    assert_eq!(prod_val, 42);
    assert_eq!(test_val, 999);
}

#[test]
fn override_with_injectable_deps() {
    // Dep is NOT injectable — it has no default construction rule.
    // The production provider supplies it via #[provide].
    #[derive(Debug, PartialEq)]
    struct Dep(i32);

    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Service(Dep, #[inject(2)] i32);

    // Production: provides Dep(1)
    #[provider]
    #[provide(Dep, Dep(1))]
    struct ProdProvider;

    // Test: override Dep to use value 100
    #[provider]
    #[provide(Dep, Dep(100))]
    struct TestProvider;

    let prod_svc: Service = ProdProvider.provide();
    assert_eq!(prod_svc.0.0, 1); // production Dep
    assert_eq!(prod_svc.1, 2); // unchanged inject

    let test_svc: Service = TestProvider.provide();
    assert_eq!(test_svc.0.0, 100); // overridden Dep
    assert_eq!(test_svc.1, 2); // unchanged inject
}

#[test]
fn override_dyn_trait_with_provide() {
    trait Greeter: core::fmt::Debug {
        fn greet(&self) -> &str;
    }

    #[derive(Debug)]
    struct RealGreeter;
    impl Greeter for RealGreeter {
        fn greet(&self) -> &str {
            "hello"
        }
    }

    #[derive(Debug)]
    struct StubGreeter;
    impl Greeter for StubGreeter {
        fn greet(&self) -> &str {
            "stub"
        }
    }

    // Production
    #[provider]
    #[provide(Box<dyn Greeter>, Box::new(RealGreeter))]
    struct ProdProvider;

    // Test
    #[provider]
    #[provide(Box<dyn Greeter>, Box::new(StubGreeter))]
    struct TestProvider;

    let prod: Box<dyn Greeter> = ProdProvider.provide();
    assert_eq!(prod.greet(), "hello");

    let test: Box<dyn Greeter> = TestProvider.provide();
    assert_eq!(test.greet(), "stub");
}

#[test]
fn override_multiple_bindings_at_once() {
    trait Database: core::fmt::Debug {
        fn query(&self) -> &str;
    }

    #[derive(Debug)]
    struct Postgres;
    impl Database for Postgres {
        fn query(&self) -> &str {
            "postgres"
        }
    }

    #[derive(Debug)]
    struct InMemoryDb;
    impl Database for InMemoryDb {
        fn query(&self) -> &str {
            "in-memory"
        }
    }

    #[injectable]
    #[derive(Debug)]
    struct AppService {
        db: Box<dyn Database>,
        cache: Box<dyn Cache>,
    }

    // Production
    #[provider]
    #[provide(Box<dyn Database>, Box::new(Postgres))]
    #[provide(Box<dyn Cache>, Box::new(RedisCache))]
    struct ProdProvider;

    // Test: override both
    #[provider]
    #[provide(Box<dyn Database>, Box::new(InMemoryDb))]
    #[provide(Box<dyn Cache>, Box::new(MockCache))]
    struct TestProvider;

    let prod_svc: AppService = ProdProvider.provide();
    assert_eq!(prod_svc.db.query(), "postgres");
    assert_eq!(prod_svc.cache.get("x"), Some("redis-value".into()));

    let test_svc: AppService = TestProvider.provide();
    assert_eq!(test_svc.db.query(), "in-memory");
    assert_eq!(test_svc.cache.get("x"), Some("mock-value".into()));
}

#[test]
fn override_with_singleton_field_sharing() {
    // Demonstrates that a test provider can hold singleton state while
    // still overriding specific bindings via #[provide].

    #[derive(Debug)]
    #[injectable]
    struct SharedConfig {
        #[inject(8080)]
        port: u16,
    }

    #[injectable]
    struct Server<'a> {
        config: &'a SharedConfig,
        cache: Box<dyn Cache>,
    }

    // Test provider: singleton config + mock cache
    #[injectable]
    #[provider]
    #[provide(Box<dyn Cache>, Box::new(MockCache))]
    struct TestProvider {
        #[singleton]
        config: SharedConfig,
    }

    #[provider]
    struct InitProvider;

    let provider = InitProvider.provide::<TestProvider>();
    let server: Server = provider.provide();
    assert_eq!(server.config.port, 8080);
    assert_eq!(server.cache.get("k"), Some("mock-value".into()));

    // A second Server gets the same config reference
    let server2: Server = provider.provide();
    assert!(core::ptr::eq(server.config, server2.config));
}

#[test]
fn override_value_type_with_provide() {
    // Override a simple value type (not a trait object)

    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Config {
        timeout_ms: i32,
        max_retries: i32,
    }

    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Client {
        config: Config,
    }

    // Production: real config
    #[provider]
    #[provide(Config, Config { timeout_ms: 5000, max_retries: 3 })]
    struct ProdProvider;

    // Test: fast config for testing
    #[provider]
    #[provide(Config, Config { timeout_ms: 1, max_retries: 0 })]
    struct TestProvider;

    let prod: Client = ProdProvider.provide();
    assert_eq!(prod.config.timeout_ms, 5000);
    assert_eq!(prod.config.max_retries, 3);

    let test: Client = TestProvider.provide();
    assert_eq!(test.config.timeout_ms, 1);
    assert_eq!(test.config.max_retries, 0);
}

#[test]
fn override_module_export_selectively() {
    // A module exports multiple types. To selectively override one export,
    // create a test provider that supplies ALL types directly via #[provide],
    // keeping original values for ones you don't want to change.

    #[injectable]
    #[module]
    #[export(u16, 8080)]
    #[export(String, "production-db".to_string())]
    struct ConfigModule;

    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Server {
        port: u16,
        db_name: String,
    }

    // Production provider uses the module for both
    #[injectable]
    #[provider]
    struct ProdProvider(#[import] ConfigModule);

    // Test provider: override db_name, replicate port directly.
    // (Cannot #[import] the module AND #[provide] one of its exports
    // because that creates conflicting Provider impls.)
    #[provider]
    #[provide(u16, 8080)]
    #[provide(String, "test-db".to_string())]
    struct TestProvider;

    #[provider]
    struct InitProvider;

    let prod = InitProvider.provide::<ProdProvider>();
    let prod_server: Server = prod.provide();
    assert_eq!(prod_server.port, 8080);
    assert_eq!(prod_server.db_name, "production-db");

    let test_server: Server = TestProvider.provide();
    assert_eq!(test_server.port, 8080); // replicated from module
    assert_eq!(test_server.db_name, "test-db"); // overridden
}

//! Integration tests combining multiple conject features together.
#![allow(dead_code)]

use conject::{
    Factory, Late, Lazy, Named, create, init, inject, injectable, key, module, provider,
};
use std::sync::Arc;

// -- Realistic app modules -----------------------------------------------

// Config module with struct-level exports
#[injectable]
#[module]
#[export(u16, 8080)]
#[export(String, "my-app".to_string())]
struct ConfigModule;

// Database module with field-level exports
#[derive(Debug, Clone)]
#[injectable]
struct DbPool(#[inject(5)] i32); // connection count

#[injectable]
#[module]
struct DbModule {
    #[export]
    pool: DbPool,
}

// Cache module depending on config
#[injectable]
#[module]
#[export(Vec<u8>, |port: u16| format!("cache://localhost:{}", port).into_bytes())]
struct CacheModule;

// -- Services using modules ----------------------------------------------

#[injectable]
struct HttpServer {
    port: u16,
    app_name: String,
}

#[injectable]
struct Repository<'a> {
    db: &'a DbPool,
}

// -- Provider combining all modules --------------------------------------

#[injectable]
#[provider]
struct AppProvider(#[import] ConfigModule, #[import] CacheModule);

// -- Tests ---------------------------------------------------------------

#[test]
fn full_app_with_init_and_multiple_modules() {
    // init! chains ConfigModule first so CacheModule can use the u16 export
    let provider: AppProvider = init!(ConfigModule, CacheModule);

    let server: HttpServer = provider.provide();
    assert_eq!(server.port, 8080);
    assert_eq!(server.app_name, "my-app");

    let cache_addr: Vec<u8> = provider.provide();
    assert_eq!(cache_addr, b"cache://localhost:8080");
}

#[test]
fn init_block_with_field_exports_and_services() {
    #[injectable]
    #[provider]
    struct FullProvider(#[import] ConfigModule, #[import] DbModule);

    init! {
        let provider: FullProvider = ConfigModule, DbModule;
    }

    let repo: Repository = provider.provide();
    assert_eq!(repo.db.0, 5);

    let server: HttpServer = provider.provide();
    assert_eq!(server.port, 8080);
}

#[test]
fn optional_deps_with_provide() {
    #[derive(Debug)]
    struct Logger {
        level: String,
    }

    #[injectable]
    struct ServiceWithOptional {
        port: u16,
        logger: Option<Logger>,
    }

    #[provider]
    #[provide(u16, 3000)]
    struct MinimalProvider;

    let svc: ServiceWithOptional = MinimalProvider.provide();
    assert_eq!(svc.port, 3000);
    assert!(svc.logger.is_none()); // Optional dep not provided
}

#[test]
fn lazy_and_inject_combined() {
    #[injectable]
    struct Config {
        #[inject(42)]
        max_connections: i32,
    }

    #[injectable]
    struct Service {
        config: Config,
        cache: Lazy<String>,
    }

    #[provider]
    struct Provider;

    let svc: Service = Provider.provide();
    assert_eq!(svc.config.max_connections, 42);
    assert!(!svc.cache.is_initialized());

    // Initialize lazily
    let val = svc
        .cache
        .get_or_init(|| format!("cache-{}", svc.config.max_connections));
    assert_eq!(val, "cache-42");
    assert!(svc.cache.is_initialized());
}

#[test]
fn post_construct_with_optional_and_inject() {
    #[injectable]
    #[post_construct(|mut s: Self| { s.computed = format!("{}:{}", s.host, s.port); s })]
    struct ServerConfig {
        #[inject("localhost".to_string())]
        host: String,
        #[inject(443)]
        port: u16,
        #[inject(String::new())]
        computed: String,
        tls_cert: Option<String>,
    }

    #[provider]
    struct Provider;

    let config: ServerConfig = Provider.provide();
    assert_eq!(config.computed, "localhost:443");
    assert!(config.tls_cert.is_none());
}

#[test]
fn circular_deps_with_late() {
    #[injectable]
    #[derive(Debug)]
    struct EventBus {
        subscribers: Late<Arc<HandlerRegistry>>,
        #[inject(0)]
        event_count: i32,
    }

    #[injectable]
    #[derive(Debug)]
    struct HandlerRegistry {
        bus: Arc<EventBus>,
    }

    #[provider]
    struct Provider;

    let bus = Arc::new(Provider.provide::<EventBus>());
    let registry = Arc::new(HandlerRegistry {
        bus: Arc::clone(&bus),
    });
    bus.subscribers.set(Arc::clone(&registry)).unwrap();

    assert!(Arc::ptr_eq(&bus.subscribers.get().unwrap().bus, &bus));
    assert_eq!(bus.event_count, 0);
}

#[test]
fn assisted_with_injected_deps() {
    #[injectable]
    struct InnerDbPool(#[inject(10)] i32);

    #[injectable]
    struct OrderService {
        db: InnerDbPool,
        #[assisted]
        customer_id: i32,
        #[assisted]
        amount: f64,
    }

    #[provider]
    struct Provider;

    let order = OrderService::create(&Provider, 42, 99.99);
    assert_eq!(order.db.0, 10);
    assert_eq!(order.customer_id, 42);
    assert_eq!(order.amount, 99.99);
}

#[test]
fn named_injection_with_multiple_sources() {
    struct PrimaryDb;
    struct ReplicaDb;

    #[injectable]
    struct DualDbService {
        #[inject(named(PrimaryDb))]
        primary: String,
        #[inject(named(ReplicaDb))]
        replica: String,
    }

    #[provider]
    #[provide(Named<PrimaryDb, String>, Named::new("postgres://primary:5432".into()))]
    #[provide(Named<ReplicaDb, String>, Named::new("postgres://replica:5432".into()))]
    struct Provider;

    let svc: DualDbService = Provider.provide();
    assert_eq!(svc.primary, "postgres://primary:5432");
    assert_eq!(svc.replica, "postgres://replica:5432");
}

#[test]
fn decorator_with_provide() {
    trait Formatter: core::fmt::Debug {
        fn format(&self, msg: &str) -> String;
    }

    #[derive(Debug)]
    struct PlainFormatter;
    impl Formatter for PlainFormatter {
        fn format(&self, msg: &str) -> String {
            msg.to_string()
        }
    }

    #[derive(Debug)]
    struct BracketFormatter(Box<dyn Formatter>);
    impl Formatter for BracketFormatter {
        fn format(&self, msg: &str) -> String {
            format!("[{}]", self.0.format(msg))
        }
    }

    #[provider]
    #[provide(Box<dyn Formatter>, Box::new(PlainFormatter))]
    #[decorate(Box<dyn Formatter>, |inner| Box::new(BracketFormatter(inner)) as Box<dyn Formatter>)]
    struct Provider;

    let fmt: Box<dyn Formatter> = Provider.provide();
    assert_eq!(fmt.format("hello"), "[hello]");
}

#[test]
fn pre_destroy_runs_cleanup() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CLEANED: AtomicBool = AtomicBool::new(false);

    #[injectable]
    #[pre_destroy(|_s: &mut Self| CLEANED.store(true, Ordering::SeqCst))]
    struct TempFile(#[inject(42)] i32);

    #[provider]
    struct Provider;

    CLEANED.store(false, Ordering::SeqCst);
    {
        let _f: TempFile = Provider.provide();
    }
    assert!(CLEANED.load(Ordering::SeqCst));
}

#[test]
fn named_string_keys_with_provider() {
    #[injectable]
    struct InfraService {
        #[inject(named("db_url"))]
        db_url: String,
        #[inject(named("cache_url"))]
        cache_url: String,
    }

    #[provider]
    #[provide(Named<key!("db_url"), String>, Named::new("postgres://localhost:5432/mydb".into()))]
    #[provide(Named<key!("cache_url"), String>, Named::new("redis://localhost:6379".into()))]
    struct Provider;

    let svc: InfraService = Provider.provide();
    assert_eq!(svc.db_url, "postgres://localhost:5432/mydb");
    assert_eq!(svc.cache_url, "redis://localhost:6379");
}

#[test]
fn singleton_with_module_and_services() {
    #[derive(Debug)]
    #[injectable]
    struct AppConfig {
        #[inject(8080)]
        port: u16,
    }

    #[injectable]
    struct WebServer<'a> {
        config: &'a AppConfig,
    }

    #[injectable]
    struct HealthEndpoint<'a> {
        config: &'a AppConfig,
    }

    #[injectable]
    #[provider]
    struct Provider {
        #[singleton]
        config: AppConfig,
    }

    #[provider]
    struct InitProvider;

    let provider = InitProvider.provide::<Provider>();
    let server: WebServer = provider.provide();
    let health: HealthEndpoint = provider.provide();

    // Both should reference the same config instance
    assert_eq!(server.config.port, 8080);
    assert_eq!(health.config.port, 8080);
    assert!(core::ptr::eq(server.config, health.config));
}

#[test]
fn factory_with_provider_and_injectable() {
    #[derive(Debug, PartialEq)]
    struct Job {
        id: u64,
    }

    #[injectable]
    struct Worker {
        job_factory: Factory<Job>,
        #[inject(String::from("worker-1"))]
        name: String,
    }

    #[provider]
    #[provide(Factory<Job>, Factory::new(|| Job { id: 1 }))]
    struct Provider;

    let worker: Worker = Provider.provide();
    assert_eq!(worker.name, "worker-1");

    let job1 = worker.job_factory.create();
    let job2 = worker.job_factory.create();
    assert_eq!(job1, Job { id: 1 });
    assert_eq!(job2, Job { id: 1 });
}

#[test]
fn mixed_named_types_and_string_keys() {
    struct Primary;

    #[injectable]
    struct MixedService {
        #[inject(named(Primary))]
        primary_url: String,
        #[inject(named("fallback_url"))]
        fallback_url: String,
        #[inject(42)]
        timeout_ms: i32,
    }

    #[provider]
    #[provide(Named<Primary, String>, Named::new("https://primary.example.com".into()))]
    #[provide(Named<key!("fallback_url"), String>, Named::new("https://fallback.example.com".into()))]
    struct Provider;

    let svc: MixedService = Provider.provide();
    assert_eq!(svc.primary_url, "https://primary.example.com");
    assert_eq!(svc.fallback_url, "https://fallback.example.com");
    assert_eq!(svc.timeout_ms, 42);
}

// ── create() — no provider needed ──────────────────────────────────

#[test]
fn create_without_provider_struct() {
    #[inject(Self { url: "postgres://localhost".into() })]
    #[derive(Debug)]
    struct Database {
        url: String,
    }

    #[inject(Self { ttl: 300 })]
    #[derive(Debug)]
    struct Cache {
        ttl: u32,
    }

    #[injectable]
    #[derive(Debug)]
    struct UserService {
        db: Database,
        cache: Cache,
    }

    // No #[provider] struct needed!
    let svc: UserService = create();
    assert_eq!(svc.db.url, "postgres://localhost");
    assert_eq!(svc.cache.ttl, 300);
}

#[test]
fn create_with_nested_deps() {
    #[inject(Self(42))]
    #[derive(Debug, PartialEq)]
    struct Config(i32);

    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Repo(Config);

    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Service(Repo);

    let svc: Service = create();
    assert_eq!(svc.0.0.0, 42);
}

#[test]
fn create_with_optional_deps() {
    #[injectable]
    #[derive(Debug)]
    struct ServiceWithOptional {
        #[inject(42)]
        port: i32,
        cache: Option<String>,
    }

    let svc: ServiceWithOptional = create();
    assert_eq!(svc.port, 42);
    assert!(svc.cache.is_none());
}

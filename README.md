<div align="center">
  <h1>conject</h1>
  <p><strong>Zero-cost, compile-time dependency injection for Rust</strong></p>
</div>
<div align="center">
  <a href="https://crates.io/crates/conject">
    <img src="https://img.shields.io/crates/v/conject.svg" alt="Crates.io version" />
  </a>
  <a href="https://crates.io/crates/conject">
    <img src="https://img.shields.io/crates/d/conject.svg" alt="Downloads" />
  </a>
  <a href="https://docs.rs/conject">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg" alt="docs.rs" />
  </a>
</div>
<br />

conject is a hard fork of [nject](https://github.com/nicolascotton/nject), a dependency injection framework that generates all wiring code **at compile time** using proc macros. The generated code is identical to what you'd write by hand — no `Box<dyn Any>`, no `HashMap` lookups, no runtime reflection. If you wire it wrong, the compiler tells you.

conject extends the original nject with: named injection (`Named<Tag, T>`, string keys via `key!()`), decorator pattern (`#[decorate]`), assisted injection (`#[assisted]`), lifecycle hooks (`#[post_construct]`, `#[pre_destroy]`, `#[async_pre_destroy]`), lazy/late initialization (`Lazy<T>`, `Late<T>`), factory types (`Factory<T>`, `DynFactory<T>`), environment variable injection (`#[inject(env("VAR"))]`), the `create()` zero-boilerplate API, `#![no_std]` support by default, and a modular macro crate refactored with [darling](https://github.com/TedDriggs/darling).

## Install

```toml
[dependencies]
conject = "0.4"
```

Requires Rust 1.85+ (edition 2024). `#![no_std]` by default.

## Quick Start — Three Levels of Power

### Level 1: Just `create()` — No Boilerplate

For the simplest use case, annotate your types and call `create()`. No provider struct needed.

```rust
use conject::{inject, injectable, create};

#[inject(Self { url: "postgres://localhost".into() })]
struct Database { url: String }

#[injectable]
struct UserService { db: Database }

fn main() {
    let svc: UserService = create();
    assert_eq!(svc.db.url, "postgres://localhost");
}
```

### Level 2: `#[provider]` — Explicit Bindings

When you need to control what gets injected (trait objects, configuration values, shared state):

```rust
use conject::{injectable, provider};

#[injectable]
struct Logger;

#[injectable]
struct UserService {
    logger: Logger,
    #[inject(8080)]
    port: u16,
}

#[provider]
struct AppProvider;

fn main() {
    let svc: UserService = AppProvider.provide();
}
```

### Level 3: `#[module]` — Modular Architecture

For large applications, modules encapsulate and export dependencies:

```rust
use conject::{injectable, provider};

mod sub {
    use conject::{injectable, module};

    #[injectable]
    struct InternalType(#[inject(123)] i32); // Not visible outside

    #[injectable]
    pub struct Facade<'a> {
        hidden: &'a InternalType
    }

    #[injectable]
    #[module]
    pub struct Module {
        #[export]
        hidden: InternalType
    }
}

#[injectable]
#[provider]
struct Provider {
    #[import]
    subModule: sub::Module
}

fn main() {
    #[provider]
    struct InitProvider;

    let provider = InitProvider.provide::<Provider>();
    let _facade = provider.provide::<sub::Facade>();
}
```

---

## Feature Guide

### Basic Injection — `#[injectable]` and `#[inject]`

`#[injectable]` marks a struct for automatic dependency resolution. Each field is resolved from the provider. For non-injectable types, use `#[inject(expr)]` to provide a value expression:

```rust
use conject::{inject, injectable, provider};

#[inject(Self { non_injectable_value: 123 })]
struct InjectableFromInjectAttr {
    non_injectable_value: i32,
}

struct NonInjectable {
    non_injectable_value: i32,
}

#[inject(|injectable_dep: InjectableFromInjectAttr| Self {
    non_injectable_value: injectable_dep.non_injectable_value + 10,
    injectable_dep
})]
struct PartiallyInjectable {
    non_injectable_value: i32,
    injectable_dep: InjectableFromInjectAttr
}

#[injectable]
struct Facade {
    dep_from_injected: InjectableFromInjectAttr,
    dep_from_partial_inject: PartiallyInjectable,
    #[inject(NonInjectable { non_injectable_value: 456 })]
    dep_from_inject_attr: NonInjectable,
    #[inject(InjectableFromInjectAttr { non_injectable_value: 789 })]
    dep_from_inject_attr_override: InjectableFromInjectAttr,
    #[inject(|injectable_dep: InjectableFromInjectAttr| PartiallyInjectable {
        non_injectable_value: 111,
        injectable_dep
    })]
    dep_from_partial_inject_attr_override: PartiallyInjectable,
}

#[provider]
struct Provider;

fn main() {
    let _facade = Provider.provide::<Facade>();
}
```

### Providers — `#[provider]` and `#[provide]`

A provider is the root of your dependency graph. Use `#[provide]` for struct-level bindings and `#[provide]` on fields for shared state:

```rust
use conject::{injectable, provider};

struct DependencyToProvide {
    value: i32,
}

struct SharedDependencyToProvide {
    value: i32,
}

#[injectable]
struct Facade<'a>(DependencyToProvide, &'a SharedDependencyToProvide);

#[provider]
#[provide(DependencyToProvide, DependencyToProvide { value: 42 })]
struct Provider {
    #[provide]
    shared: SharedDependencyToProvide
}

fn main() {
    let provider = Provider { shared: SharedDependencyToProvide { value: 123 } };
    let facade: Facade = provider.provide();
}
```

### Trait Objects (dyn Traits)

```rust
use conject::{injectable, provider};
use std::rc::Rc;

trait Greeter {
    fn greet(&self);
}

#[injectable]
struct GreeterOne;

impl Greeter for GreeterOne {
    fn greet(&self) {
        println!("Greeting");
    }
}

#[injectable]
struct Facade<'a> {
    boxed_dep: Box<dyn Greeter>,
    ref_dep: &'a dyn Greeter,
    rc_dep: Rc<dyn Greeter>,
}

#[provider]
#[provide(Box<dyn Greeter>, |greeter: GreeterOne| Box::new(greeter))]
struct Provider {
    #[provide(dyn Greeter)]
    greeter: GreeterOne,
    #[provide(Rc<dyn Greeter>, |x| x.clone())]
    rc_greeter: Rc<GreeterOne>,
}

fn main() {
    let provider = Provider {
        greeter: GreeterOne,
        rc_greeter: Rc::new(GreeterOne),
    };
    let _facade: Facade = provider.provide();
}
```

### Generics

Works with generic injectable types and generic providers:

```rust
use conject::{injectable, provider};

trait Greeter {
    fn greet(&self);
}

#[injectable]
struct DevGreeter;

impl Greeter for DevGreeter {
    fn greet(&self) {
        println!("Greeting Dev");
    }
}

#[injectable]
struct ProdGreeter;

impl Greeter for ProdGreeter {
    fn greet(&self) {
        println!("Greeting production");
    }
}

#[injectable]
struct Facade<'a> {
    dep: &'a dyn Greeter,
}

#[provider]
struct Provider<'a, T: Greeter>(#[provide(dyn Greeter)] &'a T);

fn main() {
    let _dev_facade: Facade = Provider(&DevGreeter).provide();
    let _prod_facade: Facade = Provider(&ProdGreeter).provide();
}
```

### Optional Dependencies

Fields of type `Option<T>` resolve to `None` when the provider doesn't supply `T`:

```rust
use conject::{injectable, provider};

#[injectable]
struct Service {
    #[inject(42)]
    port: i32,
    cache: Option<String>, // Not provided → None
}

#[provider]
struct Provider;

fn main() {
    let svc: Service = Provider.provide();
    assert_eq!(svc.port, 42);
    assert!(svc.cache.is_none());
}
```

### Named Injection — Disambiguating Same Types

When you need multiple values of the same type, use **type tags** or **string keys**:

#### Type Tags

```rust
use conject::{injectable, provider, Named};

struct Primary;
struct Replica;

#[injectable]
struct DualDbService {
    #[inject(named(Primary))]
    primary: String,
    #[inject(named(Replica))]
    replica: String,
}

#[provider]
#[provide(Named<Primary, String>, Named::new("postgres://primary:5432".into()))]
#[provide(Named<Replica, String>, Named::new("postgres://replica:5432".into()))]
struct Provider;

fn main() {
    let svc: DualDbService = Provider.provide();
    assert_eq!(svc.primary, "postgres://primary:5432");
    assert_eq!(svc.replica, "postgres://replica:5432");
}
```

#### String Keys

```rust
use conject::{injectable, provider, Named, key};

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

fn main() {
    let svc: InfraService = Provider.provide();
    assert_eq!(svc.db_url, "postgres://localhost:5432/mydb");
    assert_eq!(svc.cache_url, "redis://localhost:6379");
}
```

`Named<Tag, T>` is `#[repr(transparent)]` — it has the exact same memory layout as `T`. The `key!("str")` macro generates a zero-sized type via compile-time FNV-1a hashing, so string keys are also zero-cost.

### Singleton (Shared References)

Use `#[singleton]` on provider fields to share a single instance by reference:

```rust
use conject::{injectable, provider};

struct DbPool { connections: i32 }

#[injectable]
struct RepoA<'a> { db: &'a DbPool }

#[injectable]
struct RepoB<'a> { db: &'a DbPool }

#[provider]
struct AppProvider {
    #[singleton]
    db: DbPool,
}

fn main() {
    let provider = AppProvider { db: DbPool { connections: 5 } };
    let a: RepoA = provider.provide();
    let b: RepoB = provider.provide();
    // Both point to the same DbPool instance
    assert!(core::ptr::eq(a.db, b.db));
}
```

With trait objects:

```rust
use conject::{injectable, provider};

trait Greeter { fn greet(&self) -> &str; }

struct HelloGreeter;
impl Greeter for HelloGreeter {
    fn greet(&self) -> &str { "hello" }
}

#[injectable]
struct Service<'a> { greeter: &'a dyn Greeter }

#[provider]
struct AppProvider {
    #[singleton(dyn Greeter)]
    greeter: HelloGreeter,
}

fn main() {
    let p = AppProvider { greeter: HelloGreeter };
    let svc: Service = p.provide();
    assert_eq!(svc.greeter.greet(), "hello");
}
```

### Decorator Pattern

Chain transformations on a provided type using `#[decorate]`:

```rust
use conject::{injectable, provider};

trait Logger: core::fmt::Debug {
    fn log(&self, msg: &str) -> String;
}

#[derive(Debug)]
struct ConsoleLogger;
impl Logger for ConsoleLogger {
    fn log(&self, msg: &str) -> String { msg.to_string() }
}

#[derive(Debug)]
struct TimedLogger(Box<dyn Logger>);
impl Logger for TimedLogger {
    fn log(&self, msg: &str) -> String {
        format!("[timed] {}", self.0.log(msg))
    }
}

#[derive(Debug)]
struct MetricsLogger(Box<dyn Logger>);
impl Logger for MetricsLogger {
    fn log(&self, msg: &str) -> String {
        format!("[metrics] {}", self.0.log(msg))
    }
}

#[provider]
#[provide(Box<dyn Logger>, Box::new(ConsoleLogger))]
#[decorate(Box<dyn Logger>, |inner| Box::new(TimedLogger(inner)) as Box<dyn Logger>)]
#[decorate(Box<dyn Logger>, |inner| Box::new(MetricsLogger(inner)) as Box<dyn Logger>)]
struct Provider;

fn main() {
    let logger: Box<dyn Logger> = Provider.provide();
    // Decorators chain: Console → Timed → Metrics
    assert_eq!(logger.log("hello"), "[metrics] [timed] hello");
}
```

### Assisted Injection

Mix injected and caller-provided fields using `#[assisted]`. A `create()` static method is generated:

```rust
use conject::{injectable, provider};

#[injectable]
struct DbPool;

#[injectable]
#[derive(Debug, PartialEq)]
struct Order {
    db: DbPool,
    #[assisted]
    customer_id: i32,
    #[assisted]
    amount: f64,
}

#[provider]
struct AppProvider;

fn main() {
    // `db` is injected from the provider, `customer_id` and `amount` are passed directly
    let order = Order::create(&AppProvider, 42, 99.99);
    assert_eq!(order.customer_id, 42);
    assert_eq!(order.amount, 99.99);
}
```

### Lifecycle Hooks

#### `#[post_construct]` — Run after injection

```rust
use conject::{injectable, provider};

#[injectable]
#[post_construct(|mut s: Self| { s.computed = format!("{}:{}", s.host, s.port); s })]
struct ServerConfig {
    #[inject("localhost".to_string())]
    host: String,
    #[inject(443)]
    port: u16,
    #[inject(String::new())]
    computed: String,
}

#[provider]
struct Provider;

fn main() {
    let config: ServerConfig = Provider.provide();
    assert_eq!(config.computed, "localhost:443");
}
```

#### `#[pre_destroy]` — Run on drop

```rust
use conject::{injectable, provider};
use std::sync::atomic::{AtomicBool, Ordering};

static CLEANED: AtomicBool = AtomicBool::new(false);

#[injectable]
#[pre_destroy(|_s: &mut Self| CLEANED.store(true, Ordering::SeqCst))]
struct TempFile(#[inject(42)] i32);

#[provider]
struct Provider;

fn main() {
    { let _f: TempFile = Provider.provide(); }
    assert!(CLEANED.load(Ordering::SeqCst));
}
```

#### `#[async_pre_destroy]` — Async cleanup

Generates an async `destroy()` method for explicit async cleanup before drop:

```rust,no_run
use conject::{injectable, provider};

#[injectable]
#[async_pre_destroy(Self::shutdown)]
struct DbPool {
    #[inject(42)]
    connections: i32,
}

impl DbPool {
    async fn shutdown(&mut self) {
        println!("Closing {} connections", self.connections);
    }
}

#[provider]
struct Provider;

#[tokio::main]
async fn main() {
    let mut pool: DbPool = Provider.provide();
    pool.destroy().await; // Calls shutdown()
}
```

### Lazy Initialization

`Lazy<T>` fields start empty and are initialized on first access:

```rust
use conject::{injectable, provider, Lazy};

#[injectable]
struct Service {
    cache: Lazy<String>,
    #[inject(42)]
    config: i32,
}

#[provider]
struct Provider;

fn main() {
    let svc: Service = Provider.provide();
    assert!(!svc.cache.is_initialized());

    let val = svc.cache.get_or_init(|| format!("cache-{}", svc.config));
    assert_eq!(val, "cache-42");
    assert!(svc.cache.is_initialized());
}
```

### Late Initialization (Circular Dependencies)

`Late<T>` is a type alias for `Lazy<T>` — it breaks circular dependency cycles via two-phase initialization.

#### Manual wiring

```rust
use conject::{injectable, provider, Late};
use std::sync::Arc;

#[injectable]
struct EventBus {
    subscribers: Late<Arc<HandlerRegistry>>,
    #[inject(0)]
    event_count: i32,
}

#[injectable]
struct HandlerRegistry {
    bus: Arc<EventBus>,
}

#[provider]
struct Provider;

fn main() {
    let bus = Arc::new(Provider.provide::<EventBus>());
    let registry = Arc::new(HandlerRegistry { bus: Arc::clone(&bus) });
    bus.subscribers.set(Arc::clone(&registry)).unwrap();

    assert!(Arc::ptr_eq(&bus.subscribers.get().unwrap().bus, &bus));
}
```

#### Automatic wiring with `#[late_bind]`

Use `#[late_bind(target.lazy_field)]` on a `#[singleton]` provider field to automatically resolve
circular references. The `#[provider]` macro generates a `resolve_bindings()` method that calls
`.set()` for each annotated field:

```rust,ignore
use conject::{provider, Lazy};
use std::sync::Arc;

struct EventBus {
    subscribers: Lazy<Arc<HandlerRegistry>>,
    event_count: i32,
}

struct HandlerRegistry {
    bus: Arc<EventBus>,
}

#[provider]
struct AppProvider {
    #[singleton]
    bus: Arc<EventBus>,
    #[singleton]
    #[late_bind(bus.subscribers)]  // auto-sets bus.subscribers = registry.clone()
    registry: Arc<HandlerRegistry>,
}

fn main() {
    let bus = Arc::new(EventBus { subscribers: Lazy::new(), event_count: 0 });
    let registry = Arc::new(HandlerRegistry { bus: Arc::clone(&bus) });
    let provider = AppProvider { bus, registry };
    provider.resolve_bindings();  // one call wires everything

    assert!(provider.bus.subscribers.is_set());
}
```

What `cargo expand` generates for `#[late_bind(bus.subscribers)]`:

```rust,ignore
impl AppProvider {
    pub fn resolve_bindings(&self) {
        let _ = self.bus.subscribers.set(self.registry.clone());
    }
}
```

### Factory Pattern

`Factory<T>` wraps a function pointer, `DynFactory<T>` wraps a boxed closure (requires `alloc` feature):

```rust
use conject::{injectable, provider, Factory};

#[derive(Debug, PartialEq)]
struct Job { id: u64 }

#[injectable]
struct Worker {
    job_factory: Factory<Job>,
    #[inject(String::from("worker-1"))]
    name: String,
}

#[provider]
#[provide(Factory<Job>, Factory::new(|| Job { id: 1 }))]
struct Provider;

fn main() {
    let worker: Worker = Provider.provide();
    let job = worker.job_factory.create();
    assert_eq!(job, Job { id: 1 });
}
```

### Async Injection

`#[async_injectable]` supports async factory functions:

```rust,no_run
use conject::{async_injectable, provider};

#[async_injectable]
struct Config {
    #[inject(42)]
    value: i32,
}

#[provider]
struct Provider;

#[tokio::main]
async fn main() {
    let config: Config = Provider.provide_async().await;
    assert_eq!(config.value, 42);
}
```

### Environment Variable Injection

With the `env` feature, inject from environment variables at runtime:

```rust,ignore
// Cargo.toml: conject = { version = "0.4", features = ["env"] }
use conject::{injectable, provider};

#[injectable]
struct Config {
    #[inject(env("DATABASE_URL"))]
    db_url: String,
    #[inject(env("PORT", "8080".to_string()))]  // with default
    port: String,
}
```

### Module Init Macro

The `init!` macro chains module initialization, creating ephemeral providers at each step so later modules can depend on earlier exports:

```rust
use conject::{injectable, provider, module, init};

#[injectable]
#[module]
#[export(u16, 8080)]
#[export(String, "my-app".to_string())]
struct ConfigModule;

#[injectable]
#[module]
#[export(Vec<u8>, |port: u16| format!("cache://localhost:{}", port).into_bytes())]
struct CacheModule;

#[injectable]
struct HttpServer { port: u16, app_name: String }

#[injectable]
#[provider]
struct AppProvider(#[import] ConfigModule, #[import] CacheModule);

fn main() {
    // Expression form
    let provider: AppProvider = init!(ConfigModule, CacheModule);
    let server: HttpServer = provider.provide();
    assert_eq!(server.port, 8080);

    // Block form (for multiple providers)
    init! {
        let provider: AppProvider = ConfigModule, CacheModule;
    }
    let server: HttpServer = provider.provide();
    assert_eq!(server.app_name, "my-app");
}
```

### Scopes

Create child providers with scoped dependencies:

```rust
use conject::{injectable, module, provider};

#[injectable]
struct ModuleDep;

#[injectable]
#[module]
struct ScopeModule {
    #[export]
    module_dep: ModuleDep,
}

#[injectable]
struct RootDep;

#[injectable]
struct ScopeDep;

#[injectable]
struct ScopeFacade<'a> {
    root_dep: &'a RootDep,
    scope_dep: &'a ScopeDep,
    scope_module_dep: &'a ModuleDep,
}

#[injectable]
#[provider]
#[scope(ScopeDep)]
#[scope(#[import] ScopeModule)]
#[scope(other: #[arg] &'scope ScopeDep)]
#[scope(other: #[arg] &'scope ModuleDep)]
struct Provider(#[provide] RootDep);

fn main() {
    #[provider]
    struct InitProvider;

    let provider = InitProvider.provide::<Provider>();
    let scope = provider.scope();
    let scope_facade = scope.provide::<ScopeFacade>();

    let other_scope = provider.other_scope(scope_facade.scope_dep, scope_facade.scope_module_dep);
    let _other_scope_facade = other_scope.provide::<ScopeFacade>();
}
```

### Inject Providers for Post-Creation Value Injection

```rust
use conject::{injectable, provider};

#[injectable]
struct Dep(#[inject(123)] i32);

#[injectable]
struct Factory<'a> {
    dep_provider: &'a dyn conject::Provider<'a, Dep>,
}

impl<'a> Factory<'a> {
    fn create_dep(&self) -> Dep {
        self.dep_provider.provide()
    }
}

#[provider]
struct Provider;

fn main() {
    let factory = Provider.provide::<Factory>();
    let _dep = factory.create_dep();
}
```

---

## How It Works: Zero-Cost Abstraction Explained

conject's macros generate the **exact same code** you would write by hand. There is no runtime container, no type erasure, no `HashMap<TypeId, Box<dyn Any>>`. Everything resolves at compile time through trait implementations.

### What The Macros Generate

Given this code:

```rust,ignore
use conject::{inject, injectable, provider};

#[inject(Self(123))]
struct Dep1(i32);

#[injectable]
struct Service {
    dep: Dep1,
    #[inject(42)]
    port: i32,
}

#[provider]
struct MyProvider;
```

Running `cargo expand` reveals the generated code (simplified):

```rust,ignore
// #[inject(Self(123))] generates:
impl<'prov, P> Injectable<'prov, Dep1, P> for Dep1 {
    #[inline(always)]
    fn inject(_provider: &'prov P) -> Dep1 {
        Dep1(123)   // ← your expression, inlined
    }
}

// #[injectable] generates:
impl<'prov, P> Injectable<'prov, Service, P> for Service
where
    P: Provider<'prov, Dep1>,  // ← compiler-enforced dependency
{
    #[inline(always)]
    fn inject(provider: &'prov P) -> Service {
        Service {
            dep: provider.provide(),  // ← resolved via trait bound
            port: 42,                 // ← #[inject(42)] inlined
        }
    }
}

// #[provider] generates:
impl<'prov, T> Provider<'prov, T> for MyProvider
where
    T: Injectable<'prov, T, MyProvider>,
{
    #[inline(always)]
    fn provide(&'prov self) -> T {
        T::inject(self)   // ← static dispatch, fully inlined
    }
}
```

Every function is `#[inline(always)]`. The compiler sees through all the trait calls and produces the same machine code as writing `Service { dep: Dep1(123), port: 42 }` directly.

### Manual Implementation — No Macros Required

To prove this isn't magic, here's the exact same dependency injection done purely by hand. This is what the macros generate — nothing more:

```rust
use conject::{Provider, Injectable};

// ── Types (no macros) ──────────────────────────────────────────

struct Config { port: u16 }
struct Database { url: String }
struct UserService<'a> { db: &'a Database, config: &'a Config }

// ── Manual Injectable impls ────────────────────────────────────

impl<'prov, P> Injectable<'prov, Config, P> for Config {
    #[inline(always)]
    fn inject(_: &'prov P) -> Config {
        Config { port: 8080 }
    }
}

impl<'prov, P> Injectable<'prov, Database, P> for Database {
    #[inline(always)]
    fn inject(_: &'prov P) -> Database {
        Database { url: "postgres://localhost".into() }
    }
}

impl<'prov, P> Injectable<'prov, UserService<'prov>, P> for UserService<'prov>
where
    P: Provider<'prov, &'prov Config> + Provider<'prov, &'prov Database>,
{
    #[inline(always)]
    fn inject(provider: &'prov P) -> UserService<'prov> {
        UserService {
            db: provider.provide(),
            config: provider.provide(),
        }
    }
}

// ── Manual Provider impl ───────────────────────────────────────

struct AppProvider {
    config: Config,
    database: Database,
}

impl<'prov, T: Injectable<'prov, T, Self>> Provider<'prov, T> for AppProvider {
    #[inline(always)]
    fn provide(&'prov self) -> T {
        T::inject(self)
    }
}

impl<'prov> Provider<'prov, &'prov Config> for AppProvider {
    #[inline(always)]
    fn provide(&'prov self) -> &'prov Config {
        &self.config
    }
}

impl<'prov> Provider<'prov, &'prov Database> for AppProvider {
    #[inline(always)]
    fn provide(&'prov self) -> &'prov Database {
        &self.database
    }
}

// ── Usage ──────────────────────────────────────────────────────

fn main() {
    let provider = AppProvider {
        config: Config { port: 8080 },
        database: Database { url: "postgres://localhost".into() },
    };
    let svc: UserService = provider.provide();
    assert_eq!(svc.config.port, 8080);
    assert_eq!(svc.db.url, "postgres://localhost");
}
```

This is exactly what the macros generate. The only difference is that you don't have to write the boilerplate yourself. The trait bounds ensure correctness at compile time, and `#[inline(always)]` ensures the optimizer eliminates all indirection.

### What Each Feature Expands To (`cargo expand`)

Below is a summary of what every major macro/attribute generates. You can verify any of these yourself with `cargo expand`.

#### `#[inject(expr)]` on a struct
```rust,ignore
#[inject(Self(123))]
struct Dep1(i32);

// Expands to:
impl<'prov, P> Injectable<'prov, Dep1, P> for Dep1 {
    fn inject(_: &'prov P) -> Dep1 { Dep1(123) }
}
```

#### `#[injectable]` with field-level `#[inject]`
```rust,ignore
#[injectable]
struct Service { dep: Dep1, #[inject(42)] port: i32 }

// Expands to:
impl<'prov, P: Provider<'prov, Dep1>> Injectable<'prov, Service, P> for Service {
    fn inject(provider: &'prov P) -> Service {
        Service { dep: provider.provide(), port: 42 }
    }
}
```

#### `#[injectable]` with `Option<T>`, `Lazy<T>`, `Late<T>` fields
```rust,ignore
// Option<T> → None, Lazy<T> → Lazy::new(), Late<T> → Lazy::new()
impl<'prov, P> Injectable<'prov, MyStruct, P> for MyStruct {
    fn inject(provider: &'prov P) -> MyStruct {
        MyStruct { opt: None, lazy: ::conject::Lazy::new(), late: ::conject::Lazy::new() }
    }
}
```

#### `#[provider]`
```rust,ignore
#[provider]
struct MyProvider;

// Expands to:
impl<'prov, T: Injectable<'prov, T, MyProvider>> Provider<'prov, T> for MyProvider {
    fn provide(&'prov self) -> T { T::inject(self) }
}
// Plus: AsyncProvider impl, provide()/provide_async()/iter() convenience methods
```

#### `#[provide(Type, expr)]` on provider
```rust,ignore
#[provider]
#[provide(i32, 42)]
struct MyProvider;

// Expands to:
impl<'prov> Provider<'prov, i32> for MyProvider {
    fn provide(&'prov self) -> i32 { 42 }
}
```

#### `#[singleton]` on provider field
```rust,ignore
#[provider]
struct AppProvider { #[singleton] db: DbPool }

// Expands to:
impl<'prov> Provider<'prov, &'prov DbPool> for AppProvider {
    fn provide(&'prov self) -> &'prov DbPool { &self.db }
}
```

#### `#[singleton(dyn Trait)]` on provider field
```rust,ignore
#[provider]
struct AppProvider { #[singleton(dyn Greeter)] greeter: HelloGreeter }

// Expands to:
impl<'prov> Provider<'prov, &'prov dyn Greeter> for AppProvider {
    fn provide(&'prov self) -> &'prov dyn Greeter { &self.greeter }
}
```

#### `#[decorate(Type, |inner| ...)]`
```rust,ignore
#[provider]
#[provide(Box<dyn Logger>, Box::new(ConsoleLogger))]
#[decorate(Box<dyn Logger>, |inner| Box::new(TimedLogger(inner)) as Box<dyn Logger>)]
struct Provider;

// Expands to:
impl<'prov> Provider<'prov, Box<dyn Logger>> for Provider {
    fn provide(&'prov self) -> Box<dyn Logger> {
        let __conject_inner = { Box::new(ConsoleLogger) };        // base
        let __conject_inner = { let inner = __conject_inner;      // decorator 1
            Box::new(TimedLogger(inner)) as Box<dyn Logger> };
        __conject_inner
    }
}
```

#### `#[inject(named(Tag))]` (sugar for Named injection)
```rust,ignore
#[injectable]
struct Svc { #[inject(named(DbUrl))] db_url: String }

// Expands to:
impl<'prov, P: Provider<'prov, Named<DbUrl, String>>> Injectable<'prov, Svc, P> for Svc {
    fn inject(provider: &'prov P) -> Svc {
        Svc { db_url: provider.provide::<Named<DbUrl, String>>().into_inner() }
    }
}
```

#### `#[post_construct(|s: Self| ...)]`
```rust,ignore
#[injectable]
#[post_construct(|mut s: Self| { s.computed = s.a + s.b; s })]
struct Sum { #[inject(10)] a: i32, #[inject(20)] b: i32, #[inject(0)] computed: i32 }

// The Injectable::inject wraps the construction:
fn inject(provider: &'prov P) -> Sum {
    (|mut s: Sum| { s.computed = s.a + s.b; s })(Sum { a: 10, b: 20, computed: 0 })
}
```

#### `#[pre_destroy(fn)]`
```rust,ignore
#[injectable]
#[pre_destroy(Self::cleanup)]
struct Resource(#[inject(42)] i32);

// Generates a Drop impl:
impl Drop for Resource {
    fn drop(&mut self) { (Self::cleanup)(self); }
}
```

#### `#[async_pre_destroy(fn)]`
```rust,ignore
#[injectable]
#[async_pre_destroy(Self::shutdown)]
struct Pool(#[inject(42)] i32);

// Generates an async destroy() method (does NOT impl Drop):
impl Pool {
    pub async fn destroy(&mut self) { (Self::shutdown)(self).await; }
}
```

#### `#[assisted]`
```rust,ignore
#[injectable]
struct Order { db: DbPool, #[assisted] customer_id: i32, #[assisted] amount: f64 }

// Generates a static create() method instead of Injectable:
impl Order {
    pub fn create<'prov, P: Provider<'prov, DbPool>>(
        provider: &'prov P, customer_id: i32, amount: f64,
    ) -> Self {
        Order { db: provider.provide(), customer_id, amount }
    }
}
```

#### `#[module]` + `#[export]`
```rust,ignore
#[injectable]
#[module]
#[export(u16, 8080)]
struct ConfigModule;

// Generates RefInjectable (for single value) and RefIterable (for iteration):
impl<'prov, P> RefInjectable<'prov, u16, P> for ConfigModule {
    fn inject(&'prov self, _: &'prov P) -> u16 { 8080 }
}
```

#### `#[import]` on provider field
```rust,ignore
#[injectable]
#[provider]
struct AppProvider(#[import] ConfigModule);

// Generates Import trait:
impl Import<ConfigModule> for AppProvider {
    fn reference(&self) -> &ConfigModule { &self.0 }
}
// Plus Provider<u16> that delegates to the module's RefInjectable
impl<'prov> Provider<'prov, u16> for AppProvider {
    fn provide(&'prov self) -> u16 {
        RefInjectable::<u16, Self>::inject(&self.0, self)
    }
}
```

#### `#[late_bind(target.field)]`
```rust,ignore
#[provider]
struct AppProvider {
    #[singleton] bus: Arc<EventBus>,
    #[singleton] #[late_bind(bus.subscribers)] registry: Arc<HandlerRegistry>,
}

// Generates:
impl AppProvider {
    pub fn resolve_bindings(&self) {
        let _ = self.bus.subscribers.set(self.registry.clone());
    }
}
```

### Key Design Decisions

| Decision | Why |
|----------|-----|
| `#[inline(always)]` on all generated fns | Enables the compiler to fully inline the entire dependency chain — zero overhead |
| Generic `Provider` over concrete types | Monomorphization produces specialized code for each provider — no vtable dispatch |
| `Injectable` trait with provider type param | Compile-time verification that the provider can satisfy all dependencies |
| `#[repr(transparent)]` on `Named<N, V>` | Named injection adds zero runtime cost — same memory layout as the inner value |
| `OnceCell`-based `Late<T>` and `Lazy<T>` | Standard library primitives — no allocation, no locking for single-threaded use |
| `fn() -> T` for `Factory<T>` | Function pointer, not `Box<dyn Fn>` — no heap allocation |

---

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `macro` | Yes | Enables proc macros (`#[injectable]`, `#[provider]`, etc.) |
| `alloc` | No | Enables `DynFactory<T>` (requires `extern crate alloc`) |
| `std` | No | Implies `alloc`, enables `std` features |
| `env` | No | Enables `#[inject(env("VAR"))]` for env var injection (implies `std`) |

The library is `#![no_std]` by default. Core types (`Late`, `Lazy`, `Factory`, `Named`, `Key`) work without any feature flags.

## Examples

- [**Axum**](./examples/axum) — Web API with axum
- [**Actix**](./examples/actix) — Web API with actix-web
- [**Leptos**](./examples/leptos) — Full-stack web app
- [**PubSub**](./examples/pubsub) — Event-driven architecture
- [**Benchmarks**](./examples/benchmark) — Proof of zero-cost: conject vs hand-written code

## Credits

- [Syn](https://github.com/dtolnay/syn) — Rust parser for proc macros
- [Quote](https://github.com/dtolnay/quote) — Quasi-quoting for code generation
- [Darling](https://github.com/TedDriggs/darling) — Attribute parsing
- [const-fnv1a-hash](https://crates.io/crates/const-fnv1a-hash) — Compile-time hashing for string keys

## License

MIT

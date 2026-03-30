//! Realistic example: A web service with database, cache, auth, and logging.
#![allow(dead_code)]

use nject::{init, injectable, module, provider};

// -- Domain types --------------------------------------------------------

#[derive(Debug, Clone)]
struct AppConfig {
    port: u16,
    db_url: String,
    cache_ttl: u32,
}

#[derive(Debug, Clone)]
struct DbPool {
    url: String,
    max_connections: i32,
}

#[derive(Debug)]
struct CacheClient {
    ttl: u32,
}

#[derive(Debug)]
struct AuthService {
    secret: String,
}

// -- Modules -------------------------------------------------------------

#[injectable]
#[module]
#[export(AppConfig, AppConfig { port: 8080, db_url: "postgres://localhost/myapp".into(), cache_ttl: 300 })]
struct ConfigModule;

#[injectable]
#[module]
#[export(DbPool, |config: AppConfig| DbPool { url: config.db_url.clone(), max_connections: 10 })]
struct DatabaseModule;

#[injectable]
#[module]
#[export(CacheClient, |config: AppConfig| CacheClient { ttl: config.cache_ttl })]
struct CacheModule;

#[injectable]
#[module]
#[export(AuthService, AuthService { secret: "super-secret-key".into() })]
struct AuthModule;

// -- Application services ------------------------------------------------

#[injectable]
struct UserRepository {
    db: DbPool,
    cache: CacheClient,
}

#[injectable]
struct UserService {
    repo: UserRepository,
    auth: AuthService,
}

#[injectable]
struct HealthCheck {
    config: AppConfig,
    db: DbPool,
}

// -- Provider ------------------------------------------------------------

#[injectable]
#[provider]
struct AppProvider(
    #[import] ConfigModule,
    #[import] DatabaseModule,
    #[import] CacheModule,
    #[import] AuthModule,
);

fn main() {
    let provider: AppProvider = init!(ConfigModule, DatabaseModule, CacheModule, AuthModule);

    let config: AppConfig = provider.provide();
    println!("Starting {} on port {}", "MyApp", config.port);

    let user_svc: UserService = provider.provide();
    println!(
        "UserService ready: db={}, cache_ttl={}",
        user_svc.repo.db.url, user_svc.repo.cache.ttl
    );

    let health: HealthCheck = provider.provide();
    println!(
        "HealthCheck: port={}, db={}",
        health.config.port, health.db.url
    );

    println!("Auth secret: {}", user_svc.auth.secret);
}

#![no_std]
#![allow(dead_code)]
//! Realistic example: A web service with database, cache, auth, and logging.

// no_std + alloc compatible: uses String from alloc.
// `extern crate alloc` provides heap types; `extern crate std` provides the
// binary runtime. Replace std with your own in a real no_std target.
extern crate alloc;
extern crate std;

use alloc::string::String;
use conject::{init, injectable, module, provider};

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
    assert_eq!(config.port, 8080);
    assert_eq!(config.db_url, "postgres://localhost/myapp");
    assert_eq!(config.cache_ttl, 300);

    let user_svc: UserService = provider.provide();
    assert_eq!(user_svc.repo.db.url, "postgres://localhost/myapp");
    assert_eq!(user_svc.repo.cache.ttl, 300);
    assert_eq!(user_svc.auth.secret, "super-secret-key");

    let health: HealthCheck = provider.provide();
    assert_eq!(health.config.port, 8080);
    assert_eq!(health.db.url, "postgres://localhost/myapp");
}

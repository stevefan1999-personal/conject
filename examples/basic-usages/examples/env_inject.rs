//! Example: Injecting environment variables with `#[inject(env("KEY"))]`
//!
//! Demonstrates reading environment variables at construction time,
//! with optional default values when the variable is not set.

use conject::{injectable, provider};

#[injectable]
struct AppConfig {
    /// Read from the APP_HOST env var, panic if missing.
    #[inject(env("APP_HOST"))]
    host: String,

    /// Read from APP_PORT with a default fallback.
    #[inject(env("APP_PORT", "3000".to_string()))]
    port: String,

    /// A regular injected value (not from env).
    #[inject("my-app".to_string())]
    name: String,
}

#[provider]
struct AppProvider;

fn main() {
    // SAFETY: single-threaded example; no concurrent env access.
    unsafe {
        std::env::set_var("APP_HOST", "localhost");
        // APP_PORT is intentionally not set to demonstrate the default
    }

    let config: AppConfig = AppProvider.provide();

    println!("App name: {}", config.name);
    println!("Host:     {}", config.host);
    println!("Port:     {}", config.port);

    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, "3000");
    assert_eq!(config.name, "my-app");

    // Now set APP_PORT and verify it overrides the default
    // SAFETY: single-threaded example; no concurrent env access.
    unsafe { std::env::set_var("APP_PORT", "8080") };
    let config2: AppConfig = AppProvider.provide();
    assert_eq!(config2.port, "8080");

    println!("\nWith APP_PORT=8080:");
    println!("Port:     {}", config2.port);

    // Cleanup
    // SAFETY: single-threaded example; no concurrent env access.
    unsafe {
        std::env::remove_var("APP_HOST");
        std::env::remove_var("APP_PORT");
    }
}

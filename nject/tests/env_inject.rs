#![allow(dead_code)]
use nject::{injectable, provider};

#[test]
fn inject_env_should_read_env_var() {
    // SAFETY: test runs single-threaded; no other code reads TEST_PORT concurrently.
    unsafe { std::env::set_var("TEST_PORT", "8080") };

    #[injectable]
    struct Config {
        #[inject(env("TEST_PORT"))]
        port: String,
    }

    #[provider]
    struct Provider;

    let config: Config = Provider.provide();
    assert_eq!(config.port, "8080");

    // SAFETY: test cleanup.
    unsafe { std::env::remove_var("TEST_PORT") };
}

#[test]
fn inject_env_with_default_should_use_default_when_missing() {
    // SAFETY: test runs single-threaded; no other code reads MISSING_VAR concurrently.
    unsafe { std::env::remove_var("MISSING_VAR") };

    #[injectable]
    struct Config {
        #[inject(env("MISSING_VAR", "fallback".to_string()))]
        value: String,
    }

    #[provider]
    struct Provider;

    let config: Config = Provider.provide();
    assert_eq!(config.value, "fallback");
}

#[test]
fn inject_env_with_default_should_use_env_when_present() {
    // SAFETY: test runs single-threaded; no other code reads PRESENT_VAR concurrently.
    unsafe { std::env::set_var("PRESENT_VAR", "real_value") };

    #[injectable]
    struct Config {
        #[inject(env("PRESENT_VAR", "fallback".to_string()))]
        value: String,
    }

    #[provider]
    struct Provider;

    let config: Config = Provider.provide();
    assert_eq!(config.value, "real_value");

    // SAFETY: test cleanup.
    unsafe { std::env::remove_var("PRESENT_VAR") };
}

#[test]
fn inject_env_mixed_with_regular_inject() {
    // SAFETY: test runs single-threaded; no other code reads APP_NAME concurrently.
    unsafe { std::env::set_var("APP_NAME", "test-app") };

    #[injectable]
    struct Config {
        #[inject(env("APP_NAME"))]
        name: String,
        #[inject(8080)]
        port: i32,
    }

    #[provider]
    struct Provider;

    let config: Config = Provider.provide();
    assert_eq!(config.name, "test-app");
    assert_eq!(config.port, 8080);

    // SAFETY: test cleanup.
    unsafe { std::env::remove_var("APP_NAME") };
}

#![no_std]
// no_std + alloc compatible: uses String and format! from alloc.
// `extern crate alloc` provides heap types; `extern crate std` provides the
// binary runtime. Replace std with your own in a real no_std target.
#[macro_use]
extern crate alloc;
extern crate std;

use alloc::string::{String, ToString};
use conject::{injectable, provider};

#[injectable]
#[post_construct(Self::validate)]
struct Config {
    #[inject(8080)]
    port: u16,
    #[inject("localhost".to_string())]
    host: String,
}

impl Config {
    fn validate(self) -> Self {
        assert!(self.port > 0, "Port must be positive");
        self
    }
}

#[injectable]
#[post_construct(|mut s: Self| { s.description = format!("App on {}:{}", s.host, s.port); s })]
struct AppConfig {
    #[inject(3000)]
    port: u16,
    #[inject("0.0.0.0".to_string())]
    host: String,
    #[inject(String::new())]
    description: String,
}

#[provider]
struct AppProvider;

fn main() {
    let config: Config = AppProvider.provide();
    assert_eq!(config.port, 8080);
    assert_eq!(config.host, "localhost");

    let app_config: AppConfig = AppProvider.provide();
    assert_eq!(app_config.port, 3000);
    assert_eq!(app_config.host, "0.0.0.0");
    assert_eq!(app_config.description, "App on 0.0.0.0:3000");
}

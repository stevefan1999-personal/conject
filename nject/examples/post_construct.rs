use nject::{injectable, provider};

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
        println!(
            "Config validated: host={}, port={}",
            self.host, self.port
        );
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
    println!("Server: {}:{}", config.host, config.port);

    let app_config: AppConfig = AppProvider.provide();
    println!("App description: {}", app_config.description);
}

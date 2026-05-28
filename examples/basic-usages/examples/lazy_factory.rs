#![no_std]
//! Example demonstrating `Lazy<T>` and `Factory<T>` types in conject.
//!
//! Run with: `cargo run --example lazy_factory -p conject`

// no_std + alloc compatible: uses String and format! from alloc.
// `extern crate alloc` provides heap types; `extern crate std` provides the
// binary runtime. Replace std with your own in a real no_std target.
#[macro_use]
extern crate alloc;
extern crate std;

use alloc::string::String;
use conject::{Factory, Lazy, injectable, provider};

// A dependency that might be expensive to construct.
#[injectable]
struct DatabaseConnection {
    #[inject(String::from("postgres://localhost/mydb"))]
    url: String,
}

impl DatabaseConnection {
    fn query(&self, sql: &str) -> String {
        format!("Querying '{}' on {}", sql, self.url)
    }
}

// A service that uses Lazy to defer initialization of its dependency.
#[injectable]
struct UserService {
    db: Lazy<DatabaseConnection>,
}

impl UserService {
    fn get_user(&self, id: i32) -> String {
        // Initialize the database connection on first use.
        let db = self.db.get_or_init(|| DatabaseConnection {
            url: String::from("postgres://localhost/users"),
        });
        db.query(&format!("SELECT * FROM users WHERE id = {}", id))
    }
}

// A simple job type produced by a factory.
struct BackgroundJob {
    id: u64,
    name: String,
}

// A worker that uses Factory to create new job instances.
#[injectable]
struct JobProcessor {
    job_factory: Factory<BackgroundJob>,
}

impl JobProcessor {
    fn process_next(&self) -> String {
        let job = self.job_factory.create();
        format!("Processing job #{}: {}", job.id, job.name)
    }
}

// The application provider wires everything together.
#[provider]
#[provide(Factory<BackgroundJob>, Factory::new(|| BackgroundJob { id: 1, name: String::from("sync-data") }))]
struct AppProvider;

fn main() {
    let provider = AppProvider;

    // Lazy example: the database connection is not created until first use.
    let user_service: UserService = provider.provide();
    assert!(!user_service.db.is_initialized());

    let result = user_service.get_user(42);
    assert_eq!(
        result,
        "Querying 'SELECT * FROM users WHERE id = 42' on postgres://localhost/users"
    );
    assert!(user_service.db.is_initialized());

    // Factory example: each call to create() produces a new instance.
    let processor: JobProcessor = provider.provide();
    assert_eq!(processor.process_next(), "Processing job #1: sync-data");
    assert_eq!(processor.process_next(), "Processing job #1: sync-data");
}

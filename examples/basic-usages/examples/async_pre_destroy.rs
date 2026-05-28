//! Example demonstrating `#[async_pre_destroy]` for async cleanup hooks.
//!
//! **Requires `std`** — uses the tokio async runtime (`#[tokio::main]`).

use conject::{injectable, provider};

#[injectable]
#[async_pre_destroy(Self::shutdown)]
struct DbPool {
    #[inject(42)]
    connections: i32,
}

impl DbPool {
    async fn shutdown(&mut self) {
        assert_eq!(self.connections, 42);
    }
}

#[injectable]
#[async_pre_destroy(|s| { let id = s.0; async move { assert_eq!(id, 1) } })]
struct Connection(#[inject(1)] i32);

#[provider]
struct Provider;

#[tokio::main]
async fn main() {
    let mut pool: DbPool = Provider.provide();
    let mut conn: Connection = Provider.provide();
    // async_pre_destroy hooks fire here, asserting injected values
    pool.destroy().await;
    conn.destroy().await;
}

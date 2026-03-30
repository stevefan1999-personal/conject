use conject::{injectable, provider};

#[injectable]
#[async_pre_destroy(Self::shutdown)]
struct DbPool {
    #[inject(42)]
    connections: i32,
}

impl DbPool {
    async fn shutdown(&mut self) {
        println!("Async closing {} connections", self.connections);
    }
}

#[injectable]
#[async_pre_destroy(|s| async move { println!("Async dropping connection #{}", s.0) })]
struct Connection(#[inject(1)] i32);

#[provider]
struct Provider;

#[tokio::main]
async fn main() {
    println!("Creating resources...");
    let mut pool: DbPool = Provider.provide();
    let mut conn: Connection = Provider.provide();
    println!("Resources in scope.");
    pool.destroy().await;
    conn.destroy().await;
    println!("Resources cleaned up.");
}

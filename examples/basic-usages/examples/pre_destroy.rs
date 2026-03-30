use conject::{injectable, provider};

#[injectable]
#[pre_destroy(Self::cleanup)]
struct DbPool {
    #[inject(42)]
    connections: i32,
}

impl DbPool {
    fn cleanup(&mut self) {
        println!("Closing {} connections", self.connections);
    }
}

#[injectable]
#[pre_destroy(|s: &mut Connection| println!("Dropping connection #{}", s.0))]
struct Connection(#[inject(1)] i32);

#[provider]
struct Provider;

fn main() {
    println!("Creating resources...");
    {
        let _pool: DbPool = Provider.provide();
        let _conn: Connection = Provider.provide();
        println!("Resources in scope.");
    }
    println!("Resources dropped.");
}

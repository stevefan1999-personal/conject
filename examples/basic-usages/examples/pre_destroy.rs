#![no_std]
// no_std compatible: only core types (i32) — no heap allocation.
// `extern crate std` provides the binary runtime. Replace with your own
// in a real no_std target (embedded, WASM).
extern crate std;

use conject::{injectable, provider};

#[injectable]
#[pre_destroy(Self::cleanup)]
struct DbPool {
    #[inject(42)]
    connections: i32,
}

impl DbPool {
    fn cleanup(&mut self) {
        assert_eq!(self.connections, 42);
    }
}

#[injectable]
#[pre_destroy(|s: &mut Connection| assert_eq!(s.0, 1))]
struct Connection(#[inject(1)] i32);

#[provider]
struct Provider;

fn main() {
    let _pool: DbPool = Provider.provide();
    let _conn: Connection = Provider.provide();
    // pre_destroy hooks fire when _pool and _conn drop at end of main
}

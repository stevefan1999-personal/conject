#![no_std]
// no_std + alloc compatible: uses Arc from alloc::sync.
// `extern crate alloc` provides heap types; `extern crate std` provides the
// binary runtime. Replace std with your own in a real no_std target.
extern crate alloc;
extern crate std;

use alloc::sync::Arc;
use conject::{Late, injectable, provider};

#[injectable]
#[derive(Debug)]
struct ServiceA {
    dep: Late<Arc<ServiceB>>,
    #[inject(42)]
    id: i32,
}

#[derive(Debug)]
struct ServiceB {
    dep: Arc<ServiceA>,
}

#[provider]
struct Provider;

fn main() {
    // Step 1: Create ServiceA (Late<Arc<ServiceB>> starts empty)
    let a: Arc<ServiceA> = Arc::new(Provider.provide());

    // Step 2: Create ServiceB with Arc<ServiceA>
    let b = Arc::new(ServiceB {
        dep: Arc::clone(&a),
    });

    // Step 3: Complete the cycle
    a.dep.set(Arc::clone(&b)).unwrap();

    assert_eq!(a.id, 42);
    assert_eq!(b.dep.id, 42);
    assert_eq!(a.dep.get().unwrap().dep.id, 42);
}

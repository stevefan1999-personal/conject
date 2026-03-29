use nject::{injectable, provider, Late};
use std::sync::Arc;

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

    println!("A.id = {}", a.id);
    println!("B -> A.id = {}", b.dep.id);
    println!("A -> B -> A.id = {}", a.dep.dep.id);
}

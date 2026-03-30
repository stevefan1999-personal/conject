#![cfg(feature = "alloc")]
#![allow(dead_code)]
use nject::{injectable, provider, DynFactory};

#[test]
fn dynfactory_should_create_new_instances() {
    use std::sync::atomic::{AtomicI32, Ordering};
    static COUNTER: AtomicI32 = AtomicI32::new(0);

    #[provider]
    #[provide(DynFactory<i32>, DynFactory::new(|| COUNTER.fetch_add(1, Ordering::SeqCst)))]
    struct Provider;

    let factory: DynFactory<i32> = Provider.provide();
    assert_eq!(factory.create(), 0);
    assert_eq!(factory.create(), 1);
    assert_eq!(factory.create(), 2);
}

#[test]
fn dynfactory_with_injectable_type() {
    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Service(#[inject(42)] i32);

    #[provider]
    #[provide(DynFactory<Service>, DynFactory::new(|| Service(42)))]
    struct Provider;

    let factory: DynFactory<Service> = Provider.provide();
    let a = factory.create();
    let b = factory.create();
    assert_eq!(a, b);
}

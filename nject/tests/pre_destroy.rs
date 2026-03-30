#![allow(dead_code)]
use nject::{injectable, provider};
use std::sync::atomic::{AtomicBool, Ordering};

static DESTROYED: AtomicBool = AtomicBool::new(false);

#[test]
fn pre_destroy_should_run_on_drop() {
    #[injectable]
    #[pre_destroy(Self::cleanup)]
    struct Resource(#[inject(42)] i32);

    impl Resource {
        fn cleanup(&mut self) {
            DESTROYED.store(true, Ordering::SeqCst);
        }
    }

    #[provider]
    struct Provider;

    DESTROYED.store(false, Ordering::SeqCst);
    {
        let _r: Resource = Provider.provide();
        assert!(!DESTROYED.load(Ordering::SeqCst));
    }
    assert!(DESTROYED.load(Ordering::SeqCst));
}

#[test]
fn pre_destroy_with_closure_should_work() {
    static CLOSED: AtomicBool = AtomicBool::new(false);

    #[injectable]
    #[pre_destroy(|_s| CLOSED.store(true, Ordering::SeqCst))]
    struct Connection(#[inject(1)] i32);

    #[provider]
    struct Provider;

    CLOSED.store(false, Ordering::SeqCst);
    {
        let _c: Connection = Provider.provide();
    }
    assert!(CLOSED.load(Ordering::SeqCst));
}

#[test]
fn without_pre_destroy_should_not_generate_drop() {
    #[injectable]
    #[derive(Debug)]
    struct NoDrop(#[inject(42)] i32);

    #[provider]
    struct Provider;

    let n: NoDrop = Provider.provide();
    assert_eq!(n.0, 42);
}

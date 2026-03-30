use conject::{injectable, provider};
use std::sync::atomic::{AtomicBool, Ordering};

static VALIDATED: AtomicBool = AtomicBool::new(false);

#[test]
fn post_construct_should_run_after_injection() {
    #[injectable]
    #[post_construct(|s: Self| { VALIDATED.store(true, Ordering::SeqCst); s })]
    struct Validated(#[inject(42)] i32);

    #[provider]
    struct Provider;

    VALIDATED.store(false, Ordering::SeqCst);
    let v: Validated = Provider.provide();
    assert_eq!(v.0, 42);
    assert!(VALIDATED.load(Ordering::SeqCst));
}

#[test]
fn post_construct_with_method_should_work() {
    #[injectable]
    #[post_construct(Self::init)]
    struct Config {
        #[inject(8080)]
        port: u16,
        #[inject(false)]
        validated: bool,
    }

    impl Config {
        fn init(mut self) -> Self {
            self.validated = self.port > 0;
            self
        }
    }

    #[provider]
    struct Provider;

    let config: Config = Provider.provide();
    assert_eq!(config.port, 8080);
    assert!(config.validated);
}

#[test]
fn post_construct_with_transformation_should_work() {
    #[injectable]
    #[post_construct(|mut s: Self| { s.computed = s.a + s.b; s })]
    struct Sum {
        #[inject(10)]
        a: i32,
        #[inject(20)]
        b: i32,
        #[inject(0)]
        computed: i32,
    }

    #[provider]
    struct Provider;

    let sum: Sum = Provider.provide();
    assert_eq!(sum.computed, 30);
}

#[test]
fn injectable_without_post_construct_should_still_work() {
    #[injectable]
    struct Normal(#[inject(42)] i32);

    #[provider]
    struct Provider;

    let n: Normal = Provider.provide();
    assert_eq!(n.0, 42);
}

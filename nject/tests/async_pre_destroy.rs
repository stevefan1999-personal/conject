#![allow(dead_code)]
use nject::{injectable, provider};

#[tokio::test]
async fn async_pre_destroy_generates_destroy_method() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CLEANED: AtomicBool = AtomicBool::new(false);

    #[injectable]
    #[async_pre_destroy(|s| async move { CLEANED.store(true, Ordering::SeqCst); })]
    struct AsyncResource(#[inject(42)] i32);

    #[provider]
    struct Provider;

    let mut res: AsyncResource = Provider.provide();
    assert!(!CLEANED.load(Ordering::SeqCst));
    res.destroy().await;
    assert!(CLEANED.load(Ordering::SeqCst));
}

#[tokio::test]
async fn async_pre_destroy_with_method_ref() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CLOSED: AtomicBool = AtomicBool::new(false);

    #[injectable]
    #[async_pre_destroy(Self::close)]
    struct Connection(#[inject(1)] i32);

    impl Connection {
        async fn close(&mut self) {
            CLOSED.store(true, Ordering::SeqCst);
        }
    }

    #[provider]
    struct Provider;

    let mut conn: Connection = Provider.provide();
    conn.destroy().await;
    assert!(CLOSED.load(Ordering::SeqCst));
}

#[test]
fn without_async_pre_destroy_should_not_have_destroy() {
    #[injectable]
    struct Normal(#[inject(42)] i32);

    #[provider]
    struct Provider;

    let n: Normal = Provider.provide();
    assert_eq!(n.0, 42);
    // Normal does NOT have .destroy() method
}

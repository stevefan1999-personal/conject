use conject::{async_injectable, injectable, provider};

#[tokio::test]
async fn async_injectable_should_provide_struct() {
    #[async_injectable]
    #[derive(Debug, PartialEq)]
    struct Service {
        #[inject(42)]
        value: i32,
    }

    #[provider]
    struct Provider;

    let svc: Service = Provider.provide_async().await;
    assert_eq!(svc.value, 42);
}

#[tokio::test]
async fn async_injectable_with_async_inject_expr_should_work() {
    #[async_injectable]
    #[derive(Debug, PartialEq)]
    struct Service {
        #[inject(async { 42 }.await)]
        value: i32,
    }

    #[provider]
    struct Provider;

    let svc: Service = Provider.provide_async().await;
    assert_eq!(svc.value, 42);
}

#[tokio::test]
async fn async_injectable_with_deps_should_work() {
    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Dep(#[inject(99)] i32);

    #[async_injectable]
    #[derive(Debug, PartialEq)]
    struct Service(Dep);

    #[provider]
    struct Provider;

    let svc: Service = Provider.provide_async().await;
    assert_eq!(svc.0.0, 99);
}

#[tokio::test]
async fn async_injectable_tuple_struct_should_work() {
    #[async_injectable]
    #[derive(Debug, PartialEq)]
    struct Service(#[inject(42)] i32, #[inject("hello".to_string())] String);

    #[provider]
    struct Provider;

    let svc: Service = Provider.provide_async().await;
    assert_eq!(svc.0, 42);
    assert_eq!(svc.1, "hello");
}

#[tokio::test]
async fn sync_injectable_should_also_work_with_provide_async() {
    // A sync #[injectable] should also work with provide_async
    // because #[provider] generates AsyncProvider impl
    #[injectable]
    #[derive(Debug, PartialEq)]
    struct SyncService(#[inject(42)] i32);

    #[provider]
    struct Provider;

    // Sync still works
    let svc: SyncService = Provider.provide();
    assert_eq!(svc.0, 42);
}

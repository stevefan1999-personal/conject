use conject::{injectable, provider};

#[test]
fn singleton_field_should_provide_shared_reference() {
    struct DbPool {
        connections: i32,
    }

    #[provider]
    struct AppProvider {
        #[singleton]
        db: DbPool,
    }

    let provider = AppProvider {
        db: DbPool { connections: 5 },
    };
    let db1: &DbPool = provider.provide();
    let db2: &DbPool = provider.provide();
    // Both should point to the same instance
    assert_eq!(db1.connections, 5);
    assert_eq!(db2.connections, 5);
    assert!(core::ptr::eq(db1, db2));
}

#[test]
fn singleton_with_trait_should_provide_dyn_ref() {
    trait Greeter {
        fn greet(&self) -> &str;
    }

    struct HelloGreeter;
    impl Greeter for HelloGreeter {
        fn greet(&self) -> &str {
            "hello"
        }
    }

    #[provider]
    struct AppProvider {
        #[singleton(dyn Greeter)]
        greeter: HelloGreeter,
    }

    let provider = AppProvider {
        greeter: HelloGreeter,
    };
    let g: &dyn Greeter = provider.provide();
    assert_eq!(g.greet(), "hello");
}

#[test]
fn singleton_with_injectable_should_work() {
    #[injectable]
    #[derive(Debug)]
    struct Config {
        #[inject(8080)]
        port: i32,
    }

    #[injectable]
    struct Service<'a> {
        config: &'a Config,
    }

    #[injectable]
    #[provider]
    struct AppProvider {
        #[singleton]
        config: Config,
    }

    #[provider]
    struct InitProvider;

    let provider = InitProvider.provide::<AppProvider>();
    let svc: Service = provider.provide();
    assert_eq!(svc.config.port, 8080);
}

#[test]
fn mixed_singleton_and_provide_should_work() {
    struct Db {
        id: i32,
    }
    struct Cache {
        size: i32,
    }

    #[provider]
    struct AppProvider {
        #[singleton]
        db: Db,
        #[provide]
        cache: Cache,
    }

    let provider = AppProvider {
        db: Db { id: 1 },
        cache: Cache { size: 100 },
    };
    let db: &Db = provider.provide();
    let cache: &Cache = provider.provide();
    assert_eq!(db.id, 1);
    assert_eq!(cache.size, 100);
}

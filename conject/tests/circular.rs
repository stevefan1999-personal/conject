use conject::{Late, injectable, provider};
use std::sync::Arc;

#[test]
fn late_new_should_be_empty() {
    let late = Late::<i32>::new();
    assert!(!late.is_set());
    assert!(late.get().is_none());
}

#[test]
fn late_set_should_store_value() {
    let late = Late::<i32>::new();
    assert!(late.set(42).is_ok());
    assert!(late.is_set());
    assert_eq!(late.get(), Some(&42));
}

#[test]
fn late_set_twice_should_return_err() {
    let late = Late::<i32>::new();
    assert!(late.set(1).is_ok());
    assert!(late.set(2).is_err());
    assert_eq!(late.get(), Some(&1));
}

#[test]
fn late_default_should_be_empty() {
    let late: Late<String> = Default::default();
    assert!(!late.is_set());
}

#[test]
fn late_debug_empty_should_show_not_initialized() {
    let late = Late::<i32>::new();
    let debug = format!("{:?}", late);
    assert_eq!(debug, "Lazy(<not yet initialized>)");
}

#[test]
fn late_debug_set_should_show_value() {
    let late = Late::<i32>::new();
    late.set(42).unwrap();
    let debug = format!("{:?}", late);
    assert_eq!(debug, "Lazy(42)");
}

#[test]
fn late_field_should_default_to_empty_via_injectable() {
    #[injectable]
    struct Dep {
        value: Late<i32>,
    }

    #[provider]
    struct TestProvider;

    let dep: Dep = TestProvider.provide();
    assert!(!dep.value.is_set());
    dep.value.set(42).unwrap();
    assert_eq!(dep.value.get(), Some(&42));
}

#[test]
fn late_unnamed_field_should_default_to_empty_via_injectable() {
    #[injectable]
    struct Dep(Late<i32>);

    #[provider]
    struct TestProvider;

    let dep: Dep = TestProvider.provide();
    assert!(!dep.0.is_set());
    dep.0.set(99).unwrap();
    assert_eq!(dep.0.get(), Some(&99));
}

#[test]
fn late_field_mixed_with_regular_deps_should_work() {
    #[injectable]
    #[derive(Debug)]
    struct RegularDep;

    #[injectable]
    #[allow(dead_code)]
    struct Mixed {
        regular: RegularDep,
        lazy: Late<i32>,
    }

    #[provider]
    struct TestProvider;

    let mixed: Mixed = TestProvider.provide();
    assert!(!mixed.lazy.is_set());
    mixed.lazy.set(10).unwrap();
    assert_eq!(mixed.lazy.get(), Some(&10));
}

#[test]
fn late_field_with_inject_override_should_use_inject_value() {
    #[injectable]
    struct Dep {
        #[inject(Late::new())]
        value: Late<i32>,
    }

    #[provider]
    struct TestProvider;

    let dep: Dep = TestProvider.provide();
    assert!(!dep.value.is_set());
}

#[test]
fn circular_deps_with_late_should_not_overflow() {
    #[injectable]
    #[derive(Debug)]
    struct DepOne {
        dep: Late<Arc<DepTwo>>,
    }

    #[derive(Debug)]
    struct DepTwo {
        dep: Arc<DepOne>,
    }

    #[provider]
    struct TestProvider;

    // Phase 1: Create DepOne with empty Late
    let one: Arc<DepOne> = Arc::new(TestProvider.provide());

    // Phase 2: Create DepTwo with reference to one
    let two = Arc::new(DepTwo {
        dep: Arc::clone(&one),
    });

    // Phase 3: Fill in the Late reference
    one.dep.set(Arc::clone(&two)).unwrap();

    // Verify the cycle works
    assert!(Arc::ptr_eq(&one.dep.get().unwrap().dep, &one));
    assert!(one.dep.is_set());
}

#[test]
fn late_bind_should_auto_wire_circular_deps() {
    use conject::Lazy;

    #[derive(Debug)]
    struct EventBus {
        subscribers: Lazy<Arc<HandlerRegistry>>,
        event_count: i32,
    }

    #[derive(Debug)]
    struct HandlerRegistry {
        bus: Arc<EventBus>,
    }

    #[provider]
    struct AppProvider {
        #[singleton]
        bus: Arc<EventBus>,
        #[singleton]
        #[late_bind(bus.subscribers)]
        registry: Arc<HandlerRegistry>,
    }

    // Manual two-phase construction
    let bus = Arc::new(EventBus {
        subscribers: Lazy::new(),
        event_count: 0,
    });
    let registry = Arc::new(HandlerRegistry {
        bus: Arc::clone(&bus),
    });
    let provider = AppProvider { bus, registry };
    // One call resolves all late bindings — no manual .set() needed
    provider.resolve_bindings();

    assert!(provider.bus.subscribers.is_set());
    let registry_ref = provider.bus.subscribers.get().unwrap();
    assert!(Arc::ptr_eq(&registry_ref.bus, &provider.bus));
}

#[test]
fn late_bind_with_multiple_bindings() {
    use conject::Lazy;

    #[derive(Debug)]
    struct ServiceA {
        b_ref: Lazy<Arc<ServiceB>>,
        id: i32,
    }

    #[derive(Debug)]
    struct ServiceB {
        a_ref: Lazy<Arc<ServiceA>>,
        id: i32,
    }

    #[provider]
    struct AppProvider {
        #[singleton]
        #[late_bind(b.a_ref)]
        a: Arc<ServiceA>,
        #[singleton]
        #[late_bind(a.b_ref)]
        b: Arc<ServiceB>,
    }

    let a = Arc::new(ServiceA {
        b_ref: Lazy::new(),
        id: 1,
    });
    let b = Arc::new(ServiceB {
        a_ref: Lazy::new(),
        id: 2,
    });
    let provider = AppProvider { a, b };
    provider.resolve_bindings();

    assert!(provider.a.b_ref.is_set());
    assert!(provider.b.a_ref.is_set());
    assert!(Arc::ptr_eq(provider.a.b_ref.get().unwrap(), &provider.b));
    assert!(Arc::ptr_eq(provider.b.a_ref.get().unwrap(), &provider.a));
}

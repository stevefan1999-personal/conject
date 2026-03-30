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
    assert_eq!(debug, "Late(<not yet initialized>)");
}

#[test]
fn late_debug_set_should_show_value() {
    let late = Late::<i32>::new();
    late.set(42).unwrap();
    let debug = format!("{:?}", late);
    assert_eq!(debug, "Late(42)");
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

use conject::{Factory, Lazy, injectable, provider};

// --- Lazy<T> basic API tests ---

#[test]
fn lazy_new_should_be_uninitialized() {
    let lazy: Lazy<i32> = Lazy::new();
    assert!(!lazy.is_initialized());
    assert_eq!(lazy.get(), None);
}

#[test]
fn lazy_with_value_should_be_initialized() {
    let lazy = Lazy::with_value(42);
    assert!(lazy.is_initialized());
    assert_eq!(lazy.get(), Some(&42));
}

#[test]
fn lazy_get_or_init_should_initialize_on_first_call() {
    let lazy: Lazy<i32> = Lazy::new();
    let val = lazy.get_or_init(|| 42);
    assert_eq!(*val, 42);
    assert!(lazy.is_initialized());
}

#[test]
fn lazy_get_or_init_should_return_cached_value_on_second_call() {
    let lazy: Lazy<i32> = Lazy::new();
    let val1 = lazy.get_or_init(|| 42);
    let val2 = lazy.get_or_init(|| 999);
    assert_eq!(*val1, 42);
    assert_eq!(*val2, 42);
}

#[test]
fn lazy_set_should_work_when_empty() {
    let lazy: Lazy<i32> = Lazy::new();
    assert!(lazy.set(42).is_ok());
    assert_eq!(lazy.get(), Some(&42));
}

#[test]
fn lazy_set_should_fail_when_already_initialized() {
    let lazy = Lazy::with_value(42);
    assert_eq!(lazy.set(99), Err(99));
    assert_eq!(lazy.get(), Some(&42));
}

#[test]
fn lazy_into_inner_should_return_value_when_initialized() {
    let lazy = Lazy::with_value(42);
    assert_eq!(lazy.into_inner(), Some(42));
}

#[test]
fn lazy_into_inner_should_return_none_when_uninitialized() {
    let lazy: Lazy<i32> = Lazy::new();
    assert_eq!(lazy.into_inner(), None);
}

#[test]
fn lazy_get_mut_should_return_mutable_ref() {
    let mut lazy = Lazy::with_value(42);
    if let Some(val) = lazy.get_mut() {
        *val = 99;
    }
    assert_eq!(lazy.get(), Some(&99));
}

#[test]
fn lazy_default_should_be_uninitialized() {
    let lazy: Lazy<i32> = Lazy::default();
    assert!(!lazy.is_initialized());
}

#[test]
fn lazy_debug_uninitialized() {
    let lazy: Lazy<i32> = Lazy::new();
    let debug = format!("{:?}", lazy);
    assert_eq!(debug, "Lazy(<not yet initialized>)");
}

#[test]
fn lazy_debug_initialized() {
    let lazy = Lazy::with_value(42);
    let debug = format!("{:?}", lazy);
    assert_eq!(debug, "Lazy(42)");
}

#[test]
fn lazy_display_uninitialized() {
    let lazy: Lazy<i32> = Lazy::new();
    let display = format!("{}", lazy);
    assert_eq!(display, "<not yet initialized>");
}

#[test]
fn lazy_display_initialized() {
    let lazy = Lazy::with_value(42);
    let display = format!("{}", lazy);
    assert_eq!(display, "42");
}

#[test]
fn lazy_clone_initialized() {
    let lazy = Lazy::with_value(42);
    let cloned = lazy.clone();
    assert_eq!(cloned.get(), Some(&42));
}

#[test]
fn lazy_clone_uninitialized() {
    let lazy: Lazy<i32> = Lazy::new();
    let cloned = lazy.clone();
    assert!(!cloned.is_initialized());
}

#[test]
fn lazy_eq_both_uninitialized() {
    let a: Lazy<i32> = Lazy::new();
    let b: Lazy<i32> = Lazy::new();
    assert_eq!(a, b);
}

#[test]
fn lazy_eq_both_initialized_same_value() {
    let a = Lazy::with_value(42);
    let b = Lazy::with_value(42);
    assert_eq!(a, b);
}

#[test]
fn lazy_ne_different_values() {
    let a = Lazy::with_value(42);
    let b = Lazy::with_value(99);
    assert_ne!(a, b);
}

#[test]
fn lazy_ne_one_initialized_one_not() {
    let a = Lazy::with_value(42);
    let b: Lazy<i32> = Lazy::new();
    assert_ne!(a, b);
}

// --- Lazy<T> with #[injectable] macro ---

#[injectable]
#[derive(Debug, PartialEq)]
struct DepA(#[inject(42)] i32);

#[injectable]
struct ServiceWithNamedLazy {
    resource: Lazy<DepA>,
}

#[injectable]
struct ServiceWithUnnamedLazy(Lazy<DepA>);

#[injectable]
struct ServiceWithLazyAndNormalDep {
    resource: Lazy<DepA>,
    normal: DepA,
}

#[provider]
struct TestProvider;

#[test]
fn injectable_lazy_named_field_should_be_uninitialized() {
    let provider = TestProvider;
    let service: ServiceWithNamedLazy = provider.provide();
    assert!(!service.resource.is_initialized());
}

#[test]
fn injectable_lazy_unnamed_field_should_be_uninitialized() {
    let provider = TestProvider;
    let service: ServiceWithUnnamedLazy = provider.provide();
    assert!(!service.0.is_initialized());
}

#[test]
fn injectable_lazy_field_can_be_initialized_later() {
    let provider = TestProvider;
    let service: ServiceWithNamedLazy = provider.provide();
    let val = service.resource.get_or_init(|| DepA(99));
    assert_eq!(val, &DepA(99));
    assert!(service.resource.is_initialized());
}

#[test]
fn injectable_lazy_with_normal_dep_should_work() {
    let provider = TestProvider;
    let service: ServiceWithLazyAndNormalDep = provider.provide();
    assert!(!service.resource.is_initialized());
    assert_eq!(service.normal, DepA(42));
}

// --- Lazy<T> with explicit #[inject] override ---

#[injectable]
struct ServiceWithInjectOverrideLazy {
    #[inject(Lazy::with_value(DepA(123)))]
    resource: Lazy<DepA>,
}

#[test]
fn injectable_lazy_with_inject_override_should_use_override() {
    let provider = TestProvider;
    let service: ServiceWithInjectOverrideLazy = provider.provide();
    assert!(service.resource.is_initialized());
    assert_eq!(service.resource.get(), Some(&DepA(123)));
}

// --- Lazy<T> only struct (no other provider constraints) ---

#[injectable]
struct OnlyLazyFields {
    a: Lazy<i32>,
    b: Lazy<String>,
}

#[test]
fn injectable_only_lazy_fields_should_work() {
    let provider = TestProvider;
    let service: OnlyLazyFields = provider.provide();
    assert!(!service.a.is_initialized());
    assert!(!service.b.is_initialized());
}

// --- Factory<T> basic API tests ---

#[test]
fn factory_create_should_produce_value() {
    let factory = Factory::new(|| 42);
    assert_eq!(factory.create(), 42);
}

#[test]
fn factory_create_should_produce_new_instance_each_call() {
    use core::sync::atomic::{AtomicI32, Ordering};
    static COUNTER: AtomicI32 = AtomicI32::new(0);
    let factory = Factory::new(|| COUNTER.fetch_add(1, Ordering::SeqCst));
    let a = factory.create();
    let b = factory.create();
    assert_ne!(a, b);
}

#[test]
fn factory_debug() {
    let factory = Factory::new(|| 42);
    assert_eq!(format!("{:?}", factory), "Factory(<fn>)");
}

#[test]
fn factory_clone() {
    let factory = Factory::new(|| 42);
    let cloned = factory.clone();
    assert_eq!(cloned.create(), 42);
}

#[test]
fn factory_copy() {
    let factory = Factory::new(|| 42);
    let copied = factory;
    // Original is still usable (Copy)
    assert_eq!(factory.create(), 42);
    assert_eq!(copied.create(), 42);
}

// --- Factory<T> with provider ---

#[derive(Debug, PartialEq)]
struct Job(i32);

#[injectable]
struct Worker {
    job_factory: Factory<Job>,
}

#[provider]
#[provide(Factory<Job>, Factory::new(|| Job(42)))]
struct FactoryProvider;

#[test]
fn factory_provided_via_provider_should_work() {
    let provider = FactoryProvider;
    let worker: Worker = provider.provide();
    let job = worker.job_factory.create();
    assert_eq!(job, Job(42));
}

#[test]
fn factory_should_produce_distinct_instances() {
    let provider = FactoryProvider;
    let worker: Worker = provider.provide();
    let job1 = worker.job_factory.create();
    let job2 = worker.job_factory.create();
    assert_eq!(job1, Job(42));
    assert_eq!(job2, Job(42));
}

// --- Factory<T> with factory closure using provider deps ---

#[injectable]
struct DepB(#[inject(100)] i32);

#[injectable]
struct WorkerWithDeps {
    job_factory: Factory<Job>,
    dep: DepB,
}

#[test]
fn factory_with_additional_deps_should_work() {
    let provider = FactoryProvider;
    let worker: WorkerWithDeps = provider.provide();
    assert_eq!(worker.job_factory.create(), Job(42));
    assert_eq!(worker.dep.0, 100);
}

// --- Combined Lazy + Factory ---

#[injectable]
struct ServiceWithLazyAndFactory {
    lazy_resource: Lazy<DepA>,
    job_factory: Factory<Job>,
}

#[test]
fn combined_lazy_and_factory_should_work() {
    let provider = FactoryProvider;
    let service: ServiceWithLazyAndFactory = provider.provide();
    assert!(!service.lazy_resource.is_initialized());
    assert_eq!(service.job_factory.create(), Job(42));
    let val = service.lazy_resource.get_or_init(|| DepA(77));
    assert_eq!(val, &DepA(77));
}

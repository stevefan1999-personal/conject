use nject::{injectable, provider};

// -- Trait and implementations for testing --

trait Logger {
    fn log(&self, msg: &str) -> String;
}

#[injectable]
struct ConsoleLogger;
impl Logger for ConsoleLogger {
    fn log(&self, msg: &str) -> String {
        format!("console: {}", msg)
    }
}

struct TimedLogger {
    inner: Box<dyn Logger>,
}
impl Logger for TimedLogger {
    fn log(&self, msg: &str) -> String {
        let inner_result = self.inner.log(msg);
        format!("[timed] {}", inner_result)
    }
}

struct MetricsLogger {
    inner: Box<dyn Logger>,
}
impl Logger for MetricsLogger {
    fn log(&self, msg: &str) -> String {
        let inner_result = self.inner.log(msg);
        format!("[metrics] {}", inner_result)
    }
}

// -- Tests --

#[test]
fn provide_with_single_decorator_should_wrap_value() {
    #[provider]
    #[provide(Box<dyn Logger>, Box::new(ConsoleLogger))]
    #[decorate(Box<dyn Logger>, |inner| Box::new(TimedLogger { inner }) as Box<dyn Logger>)]
    struct AppProvider;

    let provider = AppProvider;
    let logger: Box<dyn Logger> = provider.provide();
    assert_eq!(logger.log("hello"), "[timed] console: hello");
}

#[test]
fn provide_with_multiple_decorators_should_chain_in_order() {
    #[provider]
    #[provide(Box<dyn Logger>, Box::new(ConsoleLogger))]
    #[decorate(Box<dyn Logger>, |inner| Box::new(TimedLogger { inner }) as Box<dyn Logger>)]
    #[decorate(Box<dyn Logger>, |inner| Box::new(MetricsLogger { inner }) as Box<dyn Logger>)]
    struct AppProvider;

    let provider = AppProvider;
    let logger: Box<dyn Logger> = provider.provide();
    // Chain: ConsoleLogger -> TimedLogger -> MetricsLogger
    assert_eq!(logger.log("hello"), "[metrics] [timed] console: hello");
}

#[test]
fn provide_without_decorator_should_work_normally() {
    #[provider]
    #[provide(Box<dyn Logger>, Box::new(ConsoleLogger))]
    struct AppProvider;

    let provider = AppProvider;
    let logger: Box<dyn Logger> = provider.provide();
    assert_eq!(logger.log("hello"), "console: hello");
}

#[test]
fn provide_with_decorator_and_factory_inputs_should_work() {
    trait Greeter {
        fn greet(&self) -> String;
    }

    #[injectable]
    struct SimpleGreeter;
    impl Greeter for SimpleGreeter {
        fn greet(&self) -> String {
            "hello".into()
        }
    }

    struct LoudGreeter {
        inner: Box<dyn Greeter>,
    }
    impl Greeter for LoudGreeter {
        fn greet(&self) -> String {
            self.inner.greet().to_uppercase()
        }
    }

    #[provider]
    #[provide(Box<dyn Greeter>, |g: SimpleGreeter| Box::new(g) as Box<dyn Greeter>)]
    #[decorate(Box<dyn Greeter>, |inner| Box::new(LoudGreeter { inner }) as Box<dyn Greeter>)]
    struct AppProvider;

    let provider = AppProvider;
    let greeter: Box<dyn Greeter> = provider.provide();
    assert_eq!(greeter.greet(), "HELLO");
}

#[test]
fn provide_with_decorator_on_concrete_type_should_work() {
    #[provider]
    #[provide(String, String::from("base"))]
    #[decorate(String, |s| format!("decorated: {}", s))]
    struct AppProvider;

    let provider = AppProvider;
    let value: String = provider.provide();
    assert_eq!(value, "decorated: base");
}

#[test]
fn provide_with_decorator_only_on_matching_type_should_not_affect_others() {
    #[provider]
    #[provide(String, String::from("a string"))]
    #[provide(i32, 42)]
    #[decorate(String, |s| format!("wrapped: {}", s))]
    struct AppProvider;

    let provider = AppProvider;
    let s: String = provider.provide();
    let n: i32 = provider.provide();
    assert_eq!(s, "wrapped: a string");
    assert_eq!(n, 42);
}

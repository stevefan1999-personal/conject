use conject::{injectable, provider};

// -- Injected dependencies --

#[injectable]
#[derive(Debug, PartialEq, Clone)]
struct DbPool;

#[injectable]
#[derive(Debug, PartialEq, Clone)]
struct Logger;

// -- Provider --

#[provider]
struct AppProvider;

// -- Test: basic assisted injection with named fields --

#[injectable]
#[derive(Debug, PartialEq)]
struct Order {
    db: DbPool,
    #[assisted]
    customer_id: i32,
    #[assisted]
    amount: f64,
}

#[test]
fn create_struct_with_named_assisted_fields_should_inject_and_pass_assisted() {
    // Given
    let provider = AppProvider;
    // When
    let order = Order::create(&provider, 42, 99.99);
    // Then
    assert_eq!(
        order,
        Order {
            db: DbPool,
            customer_id: 42,
            amount: 99.99,
        }
    );
}

// -- Test: assisted injection with tuple struct --

#[injectable]
#[derive(Debug, PartialEq)]
struct TupleOrder(DbPool, #[assisted] i32, #[assisted] f64);

#[test]
fn create_tuple_struct_with_assisted_fields_should_inject_and_pass_assisted() {
    // Given
    let provider = AppProvider;
    // When
    let order = TupleOrder::create(&provider, 42, 99.99);
    // Then
    assert_eq!(order, TupleOrder(DbPool, 42, 99.99));
}

// -- Test: mix of injected, inject-expr, and assisted fields --

#[injectable]
#[derive(Debug, PartialEq)]
struct MixedStruct {
    db: DbPool,
    #[inject(Logger)]
    logger: Logger,
    #[assisted]
    name: String,
}

#[test]
fn create_struct_with_mixed_inject_and_assisted_should_work() {
    // Given
    let provider = AppProvider;
    // When
    let result = MixedStruct::create(&provider, "test".to_string());
    // Then
    assert_eq!(
        result,
        MixedStruct {
            db: DbPool,
            logger: Logger,
            name: "test".to_string(),
        }
    );
}

// -- Test: all fields assisted (no provider deps) --

#[injectable]
#[derive(Debug, PartialEq)]
struct AllAssisted {
    #[assisted]
    x: i32,
    #[assisted]
    y: i32,
}

#[test]
fn create_struct_with_all_assisted_fields_should_only_need_provider_ref() {
    // Given
    let provider = AppProvider;
    // When
    let result = AllAssisted::create(&provider, 10, 20);
    // Then
    assert_eq!(result, AllAssisted { x: 10, y: 20 });
}

// -- Test: all fields assisted tuple struct --

#[injectable]
#[derive(Debug, PartialEq)]
struct AllAssistedTuple(#[assisted] i32, #[assisted] String);

#[test]
fn create_tuple_struct_with_all_assisted_fields_should_work() {
    // Given
    let provider = AppProvider;
    // When
    let result = AllAssistedTuple::create(&provider, 42, "hello".to_string());
    // Then
    assert_eq!(result, AllAssistedTuple(42, "hello".to_string()));
}

// -- Test: no assisted fields should still generate normal Injectable --

#[injectable]
#[derive(Debug, PartialEq)]
struct NoAssisted {
    db: DbPool,
}

#[test]
fn provide_struct_without_assisted_should_use_normal_injectable() {
    // Given
    let provider = AppProvider;
    // When
    let result: NoAssisted = provider.provide();
    // Then
    assert_eq!(result, NoAssisted { db: DbPool });
}

// -- Test: single assisted field in named struct --

#[injectable]
#[derive(Debug, PartialEq)]
struct SingleAssisted {
    db: DbPool,
    logger: Logger,
    #[assisted]
    id: u64,
}

#[test]
fn create_struct_with_single_assisted_field_should_work() {
    // Given
    let provider = AppProvider;
    // When
    let result = SingleAssisted::create(&provider, 123u64);
    // Then
    assert_eq!(
        result,
        SingleAssisted {
            db: DbPool,
            logger: Logger,
            id: 123,
        }
    );
}

// -- Test: assisted at the beginning --

#[injectable]
#[derive(Debug, PartialEq)]
struct AssistedFirst {
    #[assisted]
    name: String,
    db: DbPool,
}

#[test]
fn create_struct_with_assisted_field_first_should_work() {
    // Given
    let provider = AppProvider;
    // When
    let result = AssistedFirst::create(&provider, "first".to_string());
    // Then
    assert_eq!(
        result,
        AssistedFirst {
            name: "first".to_string(),
            db: DbPool,
        }
    );
}

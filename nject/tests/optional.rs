mod common;
pub use common::*;
use nject::{injectable, provider};

#[provider]
struct Provider;

// --- Named struct tests ---

#[injectable]
#[derive(Debug, PartialEq)]
struct StructWithOptionalNamedDep {
    dep: Option<i32>,
}

#[test]
fn provide_struct_with_optional_named_dep_should_default_to_none() {
    // Given
    let provider = Provider {};
    // When
    let value: StructWithOptionalNamedDep = provider.provide();
    // Then
    assert_eq!(value, StructWithOptionalNamedDep { dep: None });
}

#[injectable]
#[derive(Debug, PartialEq)]
struct StructWithOptionalNamedDepAndInject {
    #[inject(Some(42))]
    dep: Option<i32>,
}

#[test]
fn provide_struct_with_optional_named_dep_and_inject_should_use_annotation() {
    // Given
    let provider = Provider {};
    // When
    let value: StructWithOptionalNamedDepAndInject = provider.provide();
    // Then
    assert_eq!(
        value,
        StructWithOptionalNamedDepAndInject { dep: Some(42) }
    );
}

#[injectable]
#[derive(Debug, PartialEq)]
struct StructWithMixedDeps {
    required: StructWithoutDeps,
    optional: Option<i32>,
}

#[test]
fn provide_struct_with_mixed_deps_should_inject_required_and_default_optional() {
    // Given
    let provider = Provider {};
    // When
    let value: StructWithMixedDeps = provider.provide();
    // Then
    assert_eq!(
        value,
        StructWithMixedDeps {
            required: StructWithoutDeps,
            optional: None,
        }
    );
}

#[injectable]
#[derive(Debug, PartialEq)]
struct StructWithMultipleOptionalDeps {
    opt_a: Option<i32>,
    opt_b: Option<String>,
}

#[test]
fn provide_struct_with_multiple_optional_deps_should_all_default_to_none() {
    // Given
    let provider = Provider {};
    // When
    let value: StructWithMultipleOptionalDeps = provider.provide();
    // Then
    assert_eq!(
        value,
        StructWithMultipleOptionalDeps {
            opt_a: None,
            opt_b: None,
        }
    );
}

// --- Tuple struct tests ---

#[injectable]
#[derive(Debug, PartialEq)]
struct TupleStructWithOptionalDep(Option<i32>);

#[test]
fn provide_tuple_struct_with_optional_dep_should_default_to_none() {
    // Given
    let provider = Provider {};
    // When
    let value: TupleStructWithOptionalDep = provider.provide();
    // Then
    assert_eq!(value, TupleStructWithOptionalDep(None));
}

#[injectable]
#[derive(Debug, PartialEq)]
struct TupleStructWithOptionalDepAndInject(#[inject(Some(99))] Option<i32>);

#[test]
fn provide_tuple_struct_with_optional_dep_and_inject_should_use_annotation() {
    // Given
    let provider = Provider {};
    // When
    let value: TupleStructWithOptionalDepAndInject = provider.provide();
    // Then
    assert_eq!(value, TupleStructWithOptionalDepAndInject(Some(99)));
}

#[injectable]
#[derive(Debug, PartialEq)]
struct TupleStructWithMixedDeps(StructWithoutDeps, Option<i32>);

#[test]
fn provide_tuple_struct_with_mixed_deps_should_inject_required_and_default_optional() {
    // Given
    let provider = Provider {};
    // When
    let value: TupleStructWithMixedDeps = provider.provide();
    // Then
    assert_eq!(value, TupleStructWithMixedDeps(StructWithoutDeps, None));
}

// --- All-optional struct (no provider constraints) ---

#[injectable]
#[derive(Debug, PartialEq)]
struct AllOptionalStruct {
    a: Option<i32>,
    b: Option<String>,
}

#[test]
fn provide_all_optional_struct_should_work_with_any_provider() {
    // Given
    let provider = Provider {};
    // When
    let value: AllOptionalStruct = provider.provide();
    // Then
    assert_eq!(
        value,
        AllOptionalStruct {
            a: None,
            b: None,
        }
    );
}

// --- Optional with injectable inner type + inject override ---

#[injectable]
#[derive(Debug, PartialEq)]
struct StructWithOptionalInjectableDep {
    #[inject(Some(StructWithoutDeps))]
    dep: Option<StructWithoutDeps>,
}

#[test]
fn provide_struct_with_optional_injectable_dep_and_inject_should_use_annotation() {
    // Given
    let provider = Provider {};
    // When
    let value: StructWithOptionalInjectableDep = provider.provide();
    // Then
    assert_eq!(
        value,
        StructWithOptionalInjectableDep {
            dep: Some(StructWithoutDeps),
        }
    );
}

// --- Optional with inject factory using provider ---

#[injectable]
#[derive(Debug, PartialEq)]
struct StructWithOptionalFactoryDep {
    #[inject(|dep: StructWithoutDeps| Some(dep))]
    dep: Option<StructWithoutDeps>,
}

#[test]
fn provide_struct_with_optional_factory_dep_should_use_factory() {
    // Given
    let provider = Provider {};
    // When
    let value: StructWithOptionalFactoryDep = provider.provide();
    // Then
    assert_eq!(
        value,
        StructWithOptionalFactoryDep {
            dep: Some(StructWithoutDeps),
        }
    );
}

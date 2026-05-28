#![no_std]
#![allow(dead_code)]
//! Assisted injection example.
//!
//! Demonstrates using `#[assisted]` to mark fields that are provided by the
//! caller at creation time rather than resolved from the DI container.

// no_std compatible: only core types (i32, f64) — no heap allocation.
// `extern crate std` provides the binary runtime (panic handler, global
// allocator). In a real no_std target, replace it with your own.
extern crate std;

use conject::{injectable, provider};

/// A database pool that is managed by the DI container.
#[injectable]
#[derive(Debug)]
struct DbPool;

/// An order that combines injected dependencies with caller-provided values.
///
/// - `db` is injected from the provider automatically.
/// - `customer_id` and `amount` are provided by the caller via `Order::create`.
#[injectable]
#[derive(Debug)]
struct Order {
    db: DbPool,
    #[assisted]
    customer_id: i32,
    #[assisted]
    amount: f64,
}

#[provider]
struct AppProvider;

fn main() {
    let provider = AppProvider;

    // Use the generated `create` method: provider resolves `DbPool`,
    // while `customer_id` and `amount` are passed directly.
    let order = Order::create(&provider, 42, 99.99);

    assert_eq!(order.customer_id, 42);
    assert!((order.amount - 99.99).abs() < f64::EPSILON);
}

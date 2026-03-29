//! Assisted injection example.
//!
//! Demonstrates using `#[assisted]` to mark fields that are provided by the
//! caller at creation time rather than resolved from the DI container.

use nject::{injectable, provider};

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

    println!("Created order: {order:?}");
    // Output: Created order: Order { db: DbPool, customer_id: 42, amount: 99.99 }
}

#![no_std]
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]

use core::cell::OnceCell;

#[cfg(feature = "macro")]
pub use nject_macro::{
    InjectableHelperAttr, ModuleHelperAttr, ProviderHelperAttr, ScopeHelperAttr, inject,
    injectable, module, provider,
};

/// A late-initialized dependency for breaking circular dependency cycles.
///
/// Use `Late<T>` when two types depend on each other. One side uses `Late<T>`
/// which starts empty and is filled in after both types are constructed.
///
/// # Example
/// ```rust
/// use nject::Late;
///
/// let late = Late::<i32>::new();
/// assert!(!late.is_set());
/// late.set(42).unwrap();
/// assert_eq!(*late, 42);
/// ```
pub struct Late<T> {
    cell: OnceCell<T>,
}

impl<T> Late<T> {
    /// Create a new empty `Late<T>`.
    pub const fn new() -> Self {
        Self {
            cell: OnceCell::new(),
        }
    }

    /// Set the value. Returns `Err(value)` if already set.
    pub fn set(&self, value: T) -> Result<(), T> {
        self.cell.set(value)
    }

    /// Get a reference to the value. Panics if not yet set.
    pub fn get(&self) -> &T {
        self.cell
            .get()
            .expect("Late dependency not yet initialized")
    }

    /// Try to get a reference to the value. Returns `None` if not yet set.
    pub fn try_get(&self) -> Option<&T> {
        self.cell.get()
    }

    /// Check if the value has been set.
    pub fn is_set(&self) -> bool {
        self.cell.get().is_some()
    }
}

impl<T> core::ops::Deref for Late<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.get()
    }
}

impl<T> Default for Late<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for Late<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.cell.get() {
            Some(v) => f.debug_tuple("Late").field(v).finish(),
            None => f.write_str("Late(<not yet initialized>)"),
        }
    }
}

/// Provide a value for a specified type. Should be used with the `provide` macro for a better experience.
/// ```rust
/// use nject::{injectable, provider};
///
/// struct DependencyToProvide {
///     value: i32,
/// }
///
/// struct SharedDependencyToProvide {
///     value: i32,
/// }
///
/// #[injectable]
/// struct Facade<'a>(DependencyToProvide, &'a SharedDependencyToProvide);
///
/// #[provider]
/// #[provide(DependencyToProvide, DependencyToProvide { value: 42 })]
/// struct Provider {
///     #[provide]
///     shared: SharedDependencyToProvide
/// }
///
/// let provider = Provider { shared: SharedDependencyToProvide { value: 123 } };
/// let facade: Facade = provider.provide();
/// ```
pub trait Provider<'prov, Value> {
    fn provide(&'prov self) -> Value;
}

/// Inject dependencies for a specific type and return its value. Should be used with the `injectable` macro for a better experience.
/// ```rust
/// use nject::{injectable, provider};
///
/// struct Dependency {
///     value: i32,
/// }
///
/// #[injectable]
/// struct Facade {
///     #[inject(Dependency { value: 42 })]
///     dep: Dependency
/// }
///
/// #[provider]
/// struct Provider;
///
/// let _facade: Facade = Provider.provide();
/// ```
pub trait Injectable<'prov, Injecty, Provider> {
    fn inject(provider: &'prov Provider) -> Injecty;
}

/// Import exportations made from a module. Should be used with the `import` macro for a better experience.
/// ```rust
/// use nject::{injectable, provider};
///
/// mod sub {
///     use nject::{injectable, module};
///
///     #[injectable]
///     struct InternalType(#[inject(123)] i32); // Not visible outside of module.
///
///     #[injectable]
///     pub struct Facade<'a> {
///         hidden: &'a InternalType
///     }
///
///     #[injectable]
///     #[module]
///     pub struct Module {
///         #[export]
///         hidden: InternalType
///     }
/// }
///
/// #[injectable]
/// #[provider]
/// struct Provider {
///     #[import]
///     subModule: sub::Module
/// }
///
/// #[provider]
/// struct InitProvider;
///
/// let provider = InitProvider.provide::<Provider>();
/// let facade = provider.provide::<sub::Facade>();
/// ```
pub trait Import<Module> {
    fn reference(&self) -> &Module;
}

/// For internal purposes only. Should not be used.
#[doc(hidden)]
pub trait RefInjectable<'prov, Value, Provider> {
    fn inject(&'prov self, provider: &'prov Provider) -> Value;
}

/// For internal purposes only. Should not be used.
#[doc(hidden)]
pub trait RefIterable<'prov, Value, Provider> {
    fn inject(&'prov self, provider: &'prov Provider, index: usize) -> Value;
}

/// For internal purposes only. Should not be used.
#[doc(hidden)]
pub trait Iterable<'prov, T> {
    fn iter(&'prov self) -> impl Iterator<Item = T>;
}

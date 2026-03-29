#![no_std]
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "macro")]
pub use nject_macro::{
    InjectableHelperAttr, ModuleHelperAttr, ProviderHelperAttr, ScopeHelperAttr, inject,
    injectable, module, provider,
};

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

/// A lazily-initialized value. Wraps `core::cell::OnceCell<T>` with a convenient API.
///
/// When used as a field in an `#[injectable]` struct, `Lazy<T>` fields are automatically
/// initialized to `Lazy::new()` (empty) without requiring the provider to supply `T`.
/// The user can later initialize the value via [`get_or_init`](Lazy::get_or_init).
///
/// ```rust
/// use nject::{injectable, provider, Lazy};
///
/// #[injectable]
/// struct ExpensiveResource(#[inject(42)] i32);
///
/// #[injectable]
/// struct Service {
///     resource: Lazy<ExpensiveResource>,
/// }
///
/// #[provider]
/// struct Provider;
///
/// let provider = Provider;
/// let service: Service = provider.provide();
/// assert!(!service.resource.is_initialized());
/// let val = service.resource.get_or_init(|| ExpensiveResource(99));
/// assert!(service.resource.is_initialized());
/// ```
pub struct Lazy<T> {
    cell: core::cell::OnceCell<T>,
}

impl<T> Lazy<T> {
    /// Creates a new empty `Lazy<T>`.
    pub const fn new() -> Self {
        Self {
            cell: core::cell::OnceCell::new(),
        }
    }

    /// Creates a `Lazy<T>` that is already initialized with the given value.
    pub fn with_value(value: T) -> Self {
        let cell = core::cell::OnceCell::new();
        let _ = cell.set(value);
        Self { cell }
    }

    /// Gets the value, initializing it with `f` if it hasn't been initialized yet.
    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        self.cell.get_or_init(f)
    }

    /// Returns a reference to the value if initialized, or `None`.
    pub fn get(&self) -> Option<&T> {
        self.cell.get()
    }

    /// Returns a mutable reference to the value if initialized, or `None`.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.cell.get_mut()
    }

    /// Returns `true` if the value has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.cell.get().is_some()
    }

    /// Sets the value if it hasn't been initialized yet.
    /// Returns `Ok(())` if the value was set, or `Err(value)` if it was already initialized.
    pub fn set(&self, value: T) -> Result<(), T> {
        self.cell.set(value)
    }

    /// Consumes the `Lazy<T>` and returns the inner value if initialized.
    pub fn into_inner(self) -> Option<T> {
        self.cell.into_inner()
    }
}

impl<T> Default for Lazy<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for Lazy<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.cell.get() {
            Some(v) => f.debug_tuple("Lazy").field(v).finish(),
            None => f.write_str("Lazy(<not yet initialized>)"),
        }
    }
}

impl<T: core::fmt::Display> core::fmt::Display for Lazy<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.cell.get() {
            Some(v) => core::fmt::Display::fmt(v, f),
            None => f.write_str("<not yet initialized>"),
        }
    }
}

impl<T: Clone> Clone for Lazy<T> {
    fn clone(&self) -> Self {
        match self.cell.get() {
            Some(v) => Self::with_value(v.clone()),
            None => Self::new(),
        }
    }
}

impl<T: PartialEq> PartialEq for Lazy<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cell.get() == other.cell.get()
    }
}

impl<T: Eq> Eq for Lazy<T> {}

/// A factory that creates new instances of `T` each time [`create`](Factory::create) is called.
///
/// `Factory<T>` wraps a function pointer (`fn() -> T`) and can be provided via `#[provide]`
/// on a `#[provider]` struct.
///
/// ```rust
/// use nject::{injectable, provider, Factory};
///
/// #[derive(Debug, PartialEq)]
/// struct Job(i32);
///
/// #[injectable]
/// struct Worker {
///     job_factory: Factory<Job>,
/// }
///
/// #[provider]
/// #[provide(Factory<Job>, Factory::new(|| Job(42)))]
/// struct Provider;
///
/// let provider = Provider;
/// let worker: Worker = provider.provide();
/// let job1 = worker.job_factory.create();
/// let job2 = worker.job_factory.create();
/// assert_eq!(job1, Job(42));
/// assert_eq!(job2, Job(42));
/// ```
pub struct Factory<T>(fn() -> T);

impl<T> Factory<T> {
    /// Creates a new factory from a function pointer.
    pub const fn new(f: fn() -> T) -> Self {
        Self(f)
    }

    /// Creates a new instance of `T` by calling the factory function.
    pub fn create(&self) -> T {
        (self.0)()
    }
}

impl<T> core::fmt::Debug for Factory<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Factory(<fn>)")
    }
}

impl<T> Clone for Factory<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T> Copy for Factory<T> {}

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

#![no_std]
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]

use core::cell::OnceCell;

#[cfg(feature = "macro")]
pub use nject_macro::{
    InjectableHelperAttr, ModuleHelperAttr, ProviderHelperAttr, ScopeHelperAttr, async_injectable,
    init, inject, injectable, key, module, provider,
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

/// A zero-cost wrapper that tags a `Value` with a `Name` type for named injection.
///
/// This allows multiple dependencies of the same type to coexist in a provider
/// by distinguishing them via a phantom type tag. At runtime, `Named<Name, Value>`
/// has the exact same memory layout as `Value` due to `#[repr(transparent)]`.
///
/// ```rust
/// use nject::{injectable, provider, Named};
///
/// // Define zero-sized tag types
/// struct DbUrl;
/// struct ApiKey;
///
/// #[provider]
/// #[provide(Named<DbUrl, String>, Named::new("postgres://localhost".into()))]
/// #[provide(Named<ApiKey, String>, Named::new("secret-key".into()))]
/// struct AppProvider;
///
/// let provider = AppProvider;
/// let db_url: Named<DbUrl, String> = provider.provide();
/// let api_key: Named<ApiKey, String> = provider.provide();
/// assert_eq!(*db_url, "postgres://localhost");
/// assert_eq!(*api_key, "secret-key");
/// ```
#[repr(transparent)]
pub struct Named<Name, Value> {
    /// The wrapped value.
    pub value: Value,
    _name: core::marker::PhantomData<Name>,
}

impl<Name, Value> Named<Name, Value> {
    /// Create a new `Named` value with the given tag.
    #[inline]
    pub const fn new(value: Value) -> Self {
        Self {
            value,
            _name: core::marker::PhantomData,
        }
    }

    /// Unwrap the `Named` value, discarding the tag.
    #[inline]
    pub fn into_inner(self) -> Value {
        self.value
    }
}

impl<Name, Value: core::fmt::Debug> core::fmt::Debug for Named<Name, Value> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.value.fmt(f)
    }
}

impl<Name, Value: Clone> Clone for Named<Name, Value> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.value.clone())
    }
}

impl<Name, Value: Copy> Copy for Named<Name, Value> {}

impl<Name, Value: PartialEq> PartialEq for Named<Name, Value> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<Name, Value: Eq> Eq for Named<Name, Value> {}

impl<Name, Value> core::ops::Deref for Named<Name, Value> {
    type Target = Value;

    #[inline]
    fn deref(&self) -> &Value {
        &self.value
    }
}

impl<Name, Value> core::ops::DerefMut for Named<Name, Value> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Value {
        &mut self.value
    }
}

impl<Name, Value> From<Value> for Named<Name, Value> {
    #[inline]
    fn from(value: Value) -> Self {
        Self::new(value)
    }
}

/// A zero-sized type tag generated from a string key via FNV-1a hash.
///
/// Used with [`Named`] for string-based named injection. The `key!` macro
/// provides convenient syntax: `key!("db_url")` expands to `Key<{HASH}>`.
///
/// ```rust
/// use nject::{Named, Key, str_key_hash};
///
/// // These are equivalent:
/// type A = Named<Key<{str_key_hash("db_url")}>, String>;
///
/// let a: A = Named::new("hello".into());
/// assert_eq!(*a, "hello");
/// ```
pub struct Key<const K: u128>;

/// Compute an FNV-1a 128-bit hash of a string at compile time.
///
/// This uses the same algorithm as the internal module hashing,
/// producing a deterministic `u128` from any `&str`.
pub const fn str_key_hash(s: &str) -> u128 {
    // FNV-1a parameters for 128-bit
    // https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function#FNV_hash_parameters
    const FNV_OFFSET_BASIS: u128 = 0x6c62272e07bb014262b821756295c58d;
    const FNV_PRIME: u128 = 0x00000100000001b3;

    let bytes = s.as_bytes();
    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u128;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
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

/// Async version of [`Provider`]. Provide a value asynchronously.
/// ```rust
/// use nject::{async_injectable, provider};
///
/// #[async_injectable]
/// struct Service {
///     #[inject(42)]
///     value: i32,
/// }
///
/// #[provider]
/// struct Provider;
///
/// # tokio_test::block_on(async {
/// let svc: Service = Provider.provide_async().await;
/// # });
/// ```
pub trait AsyncProvider<'prov, Value> {
    fn provide(&'prov self) -> impl core::future::Future<Output = Value>;
}

/// Async version of [`Injectable`]. Inject dependencies asynchronously.
/// ```rust
/// use nject::{async_injectable, provider};
///
/// #[async_injectable]
/// struct Service {
///     #[inject(42)]
///     value: i32,
/// }
///
/// #[provider]
/// struct Provider;
///
/// # tokio_test::block_on(async {
/// let svc: Service = Provider.provide_async().await;
/// # });
/// ```
pub trait AsyncInjectable<'prov, Injecty, Provider> {
    fn inject(provider: &'prov Provider) -> impl core::future::Future<Output = Injecty>;
}

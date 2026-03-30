#![feature(prelude_import)]
#![allow(dead_code)]
//! Example demonstrating async dependency injection with conject.
//!
//! Run with: `cargo run --example async_init -p conject`
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use conject::{async_injectable, injectable, provider};
/// A configuration value that is synchronously injectable.
struct Config {
    #[inject("localhost:5432".to_string())]
    db_url: String,
    #[inject(5)]
    max_retries: i32,
}
#[automatically_derived]
impl ::core::fmt::Debug for Config {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "Config",
            "db_url",
            &self.db_url,
            "max_retries",
            &&self.max_retries,
        )
    }
}
impl<'prov, ConjectProvider> ::conject::Injectable<'prov, Config, ConjectProvider>
for Config {
    #[inline(always)]
    fn inject(provider: &'prov ConjectProvider) -> Config {
        Config {
            db_url: "localhost:5432".to_string(),
            max_retries: 5,
        }
    }
}
impl<'prov, ConjectProvider> ::conject::AsyncInjectable<'prov, Config, ConjectProvider>
for Config
where
    ConjectProvider:,
{
    #[inline(always)]
    fn inject(
        provider: &'prov ConjectProvider,
    ) -> impl ::core::future::Future<Output = Config> {
        ::core::future::ready(
            <Self as ::conject::Injectable<
                'prov,
                Config,
                ConjectProvider,
            >>::inject(provider),
        )
    }
}
/// A database pool that requires async initialization.
struct DbPool {
    #[inject("connected-pool".to_string())]
    connection: String,
}
#[automatically_derived]
impl ::core::fmt::Debug for DbPool {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field1_finish(
            f,
            "DbPool",
            "connection",
            &&self.connection,
        )
    }
}
impl<'prov, ConjectProvider> ::conject::AsyncInjectable<'prov, DbPool, ConjectProvider>
for DbPool
where
    ConjectProvider:,
{
    #[inline(always)]
    fn inject(
        provider: &'prov ConjectProvider,
    ) -> impl ::core::future::Future<Output = DbPool> {
        async move {
            DbPool {
                connection: { "connected-pool".to_string() },
            }
        }
    }
}
/// A service that depends on both sync and async dependencies.
struct AppService {
    config: Config,
    pool: DbPool,
    #[inject("app-v1".to_string())]
    version: String,
}
#[automatically_derived]
impl ::core::fmt::Debug for AppService {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field3_finish(
            f,
            "AppService",
            "config",
            &self.config,
            "pool",
            &self.pool,
            "version",
            &&self.version,
        )
    }
}
impl<
    'prov,
    ConjectProvider,
> ::conject::AsyncInjectable<'prov, AppService, ConjectProvider> for AppService
where
    ConjectProvider: ::conject::AsyncProvider<'prov, Config>
        + ::conject::AsyncProvider<'prov, DbPool>,
{
    #[inline(always)]
    fn inject(
        provider: &'prov ConjectProvider,
    ) -> impl ::core::future::Future<Output = AppService> {
        async move {
            AppService {
                config: <ConjectProvider as ::conject::AsyncProvider<
                    'prov,
                    Config,
                >>::provide(provider)
                    .await,
                pool: <ConjectProvider as ::conject::AsyncProvider<
                    'prov,
                    DbPool,
                >>::provide(provider)
                    .await,
                version: { "app-v1".to_string() },
            }
        }
    }
}
/// The provider that wires everything together.
struct AppProvider;
impl<'prov, Conjecty> ::conject::Provider<'prov, Conjecty> for AppProvider
where
    Conjecty: ::conject::Injectable<'prov, Conjecty, AppProvider>,
{
    #[inline(always)]
    fn provide(&'prov self) -> Conjecty {
        Conjecty::inject(self)
    }
}
impl<
    'prov,
    Conjecty,
> ::conject::Provider<'prov, &'prov dyn ::conject::Provider<'prov, Conjecty>>
for AppProvider
where
    Self: ::conject::Provider<'prov, Conjecty>,
{
    #[inline(always)]
    fn provide(&'prov self) -> &'prov dyn ::conject::Provider<'prov, Conjecty> {
        self
    }
}
impl<'prov, Conjecty> ::conject::AsyncProvider<'prov, Conjecty> for AppProvider
where
    Conjecty: ::conject::AsyncInjectable<'prov, Conjecty, AppProvider>,
{
    #[inline(always)]
    fn provide(&'prov self) -> impl ::core::future::Future<Output = Conjecty> {
        Conjecty::inject(self)
    }
}
impl AppProvider {
    #[inline(always)]
    pub fn provide<'prov, Conjecty>(&'prov self) -> Conjecty
    where
        Self: ::conject::Provider<'prov, Conjecty>,
    {
        <Self as ::conject::Provider<'prov, Conjecty>>::provide(self)
    }
    #[inline(always)]
    pub fn provide_async<'prov, Conjecty>(
        &'prov self,
    ) -> impl ::core::future::Future<Output = Conjecty>
    where
        Self: ::conject::AsyncProvider<'prov, Conjecty>,
    {
        <Self as ::conject::AsyncProvider<'prov, Conjecty>>::provide(self)
    }
    #[inline(always)]
    pub fn iter<'prov, Value>(
        &'prov self,
    ) -> impl ::core::iter::Iterator<Item = Value> + use<'prov, Value>
    where
        Self: ::conject::Iterable<'prov, Value>,
    {
        ::conject::Iterable::<'prov, Value>::iter(self)
    }
}
fn main() {
    let body = async {
        let provider = AppProvider;
        let service: AppService = provider.provide_async().await;
        {
            ::std::io::_print(format_args!("Service: {0:#?}\n", service));
        };
        let config: Config = provider.provide();
        {
            ::std::io::_print(format_args!("Config: {0:#?}\n", config));
        };
    };
    let body = {
        if false {
            let _: &dyn ::core::future::Future<Output = ()> = &body;
        }
        body
    };
    #[allow(
        clippy::expect_used,
        clippy::diverging_sub_expression,
        clippy::needless_return,
        clippy::unwrap_in_result
    )]
    {
        use tokio::runtime::Builder;
        return Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(body);
    }
}

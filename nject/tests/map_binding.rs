#![cfg(feature = "std")]
#![allow(dead_code)]
use nject::{injectable, module, provider};
use std::collections::HashMap;

/// Demonstrates collecting multiple module exports into a HashMap.
///
/// Since modules export typed iterables via `provider.iter::<T>()`,
/// we can export string references from multiple modules and collect
/// them into a map keyed by their index.
#[test]
fn string_exports_can_be_collected_as_map() {
    #[module]
    #[injectable]
    #[export(&'prov str, "auth")]
    #[export(&'prov str, "log")]
    #[export(&'prov str, "metrics")]
    struct HandlerNames;

    #[injectable]
    #[provider]
    struct AppProvider(#[import] HandlerNames);

    #[provider]
    struct InitProvider;

    let provider = InitProvider.provide::<AppProvider>();

    // Collect iterable exports into a HashMap keyed by name (identity map).
    let map: HashMap<&str, usize> = provider
        .iter::<&str>()
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect();

    assert_eq!(map.len(), 3);
    assert_eq!(map["auth"], 0);
    assert_eq!(map["log"], 1);
    assert_eq!(map["metrics"], 2);
}

/// Demonstrates collecting simple value exports from different modules into a HashMap.
#[test]
fn exports_from_multiple_modules_can_be_collected_as_map() {
    #[module]
    #[injectable]
    #[export(&'prov str, "first")]
    struct FirstModule;

    #[module]
    #[injectable]
    #[export(&'prov str, "second")]
    struct SecondModule;

    #[injectable]
    #[provider]
    struct AppProvider(#[import] FirstModule, #[import] SecondModule);

    #[provider]
    struct InitProvider;

    let provider = InitProvider.provide::<AppProvider>();

    let map: HashMap<&str, usize> = provider
        .iter::<&str>()
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect();

    assert_eq!(map.len(), 2);
    assert!(map.contains_key("first"));
    assert!(map.contains_key("second"));
}

/// Demonstrates collecting integer exports into a HashMap.
#[test]
fn integer_exports_can_be_collected_as_map() {
    #[module]
    #[injectable]
    #[export(i32, 10)]
    #[export(i32, 20)]
    #[export(i32, 30)]
    struct NumberModule;

    #[injectable]
    #[provider]
    struct AppProvider(#[import] NumberModule);

    #[provider]
    struct InitProvider;

    let provider = InitProvider.provide::<AppProvider>();

    // Collect into a map using index as key.
    let map: HashMap<usize, i32> = provider.iter::<i32>().enumerate().collect();

    assert_eq!(map.len(), 3);
    assert_eq!(map[&0], 10);
    assert_eq!(map[&1], 20);
    assert_eq!(map[&2], 30);
}

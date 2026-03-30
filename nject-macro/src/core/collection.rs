use std::collections::HashMap;
use itertools::Itertools;

/// Groups items by key, preserving insertion order within groups.
pub fn group_by<T, K, F>(iter: impl Iterator<Item = T>, key: F) -> HashMap<K, Vec<T>>
where
    F: Fn(&T) -> K,
    K: std::hash::Hash + Eq,
{
    iter.into_group_map_by(key)
}

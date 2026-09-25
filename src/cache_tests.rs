//! Tests for the cache layers.

use super::Cache;
use crate::error::Error;
use std::cell::Cell;

fn temp_cache() -> (tempfile::TempDir, Cache) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache {
        root: dir.path().to_path_buf(),
        repo_key: "testrepo".into(),
        enabled: true,
        hits: Cell::new(0),
        misses: Cell::new(0),
    };
    (dir, cache)
}

#[test]
fn round_trips_and_counts_hits() {
    let (_dir, cache) = temp_cache();

    assert!(cache.load::<Vec<String>>("style", "k1").is_none());

    cache.store("style", "k1", &vec!["a".to_string()]).unwrap();
    let loaded: Vec<String> = cache.load("style", "k1").unwrap();
    assert_eq!(loaded, vec!["a"]);
    assert_eq!(cache.hits(), 1);
}

#[test]
fn get_or_compute_only_computes_once() {
    let (_dir, cache) = temp_cache();
    let first = cache
        .get_or_compute("facts", "k2", || Ok::<_, Error>(7u32))
        .unwrap();
    assert_eq!(first, 7);
    assert_eq!(cache.misses(), 1);

    let second = cache
        .get_or_compute("facts", "k2", || Ok::<_, Error>(0u32))
        .unwrap();
    assert_eq!(second, 7, "second call must come from cache");
    assert_eq!(cache.hits(), 1);
}

#[test]
fn corrupt_entries_are_dropped_and_recomputed() {
    let (_dir, cache) = temp_cache();
    let path = cache.entry_path("facts", "k3");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{not json").unwrap();

    let value = cache
        .get_or_compute("facts", "k3", || Ok::<_, Error>(1u8))
        .unwrap();
    assert_eq!(value, 1);
    assert_eq!(cache.misses(), 1);
}

#[test]
fn stats_and_clean_see_the_entries() {
    let (_dir, cache) = temp_cache();
    cache.store("style", "a", &1u8).unwrap();
    cache.store("facts", "b", &2u8).unwrap();

    let stats = cache.stats();
    assert_eq!(stats.len(), 2);
    assert!(
        stats
            .iter()
            .any(|(ns, count, _)| ns == "style" && *count == 1)
    );

    let (entries, bytes) = cache.clean().unwrap();
    assert_eq!(entries, 2);
    assert!(bytes > 0);
    assert!(cache.stats().is_empty());
}

#[test]
fn disabled_cache_never_touches_disk() {
    let cache = Cache::new("/tmp/definitely-not-used", false);
    cache.store("style", "k", &1u8).unwrap();
    assert!(cache.load::<u8>("style", "k").is_none());
}

#[test]
fn different_repos_do_not_collide() {
    let a = Cache::new("/tmp/repo-a", true);
    let b = Cache::new("/tmp/repo-b", true);
    assert_ne!(a.repo_dir(), b.repo_dir());
    assert_eq!(a.repo_dir(), Cache::new("/tmp/repo-a", true).repo_dir());
}

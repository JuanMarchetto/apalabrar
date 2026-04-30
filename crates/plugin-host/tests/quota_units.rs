//! [`Quota`] surface tests.

use apalabrar_plugin_host::Quota;

#[test]
fn new_stores_values() {
    let q = Quota::new(42, 7);
    assert_eq!(q.fuel, 42);
    assert_eq!(q.memory_pages, 7);
}

#[test]
fn small_is_one_million_fuel_one_mib() {
    let q = Quota::small();
    assert_eq!(q.fuel, 1_000_000);
    assert_eq!(q.memory_pages, 16);
}

#[test]
fn large_is_one_hundred_million_fuel_four_mib() {
    let q = Quota::large();
    assert_eq!(q.fuel, 100_000_000);
    assert_eq!(q.memory_pages, 64);
}

#[test]
fn default_is_small() {
    let d = Quota::default();
    let s = Quota::small();
    assert_eq!(d, s);
}

#[test]
fn small_is_strictly_smaller_than_large() {
    assert!(Quota::small().fuel < Quota::large().fuel);
    assert!(Quota::small().memory_pages < Quota::large().memory_pages);
}

#[test]
fn quota_is_copy() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<Quota>();
}

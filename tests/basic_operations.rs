use cuckoo_hashmap::CuckooHashMap;

#[test]
fn test_new_map_is_empty() {
    let map: CuckooHashMap<i32, String> = CuckooHashMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn test_insert_and_get() {
    let map = CuckooHashMap::new();

    // Insert a single value
    let result = map.insert(1, "one");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None); // No previous value

    // Retrieve the value
    assert_eq!(map.get(&1), Some("one"));
    assert_eq!(map.len(), 1);
    assert!(!map.is_empty());
}

#[test]
fn test_insert_multiple() {
    let map = CuckooHashMap::new();

    // Insert multiple values
    for i in 0..100 {
        let result = map.insert(i, i * 10);
        assert!(result.is_ok());
    }

    assert_eq!(map.len(), 100);

    // Verify all values
    for i in 0..100 {
        assert_eq!(map.get(&i), Some(i * 10));
    }
}

#[test]
fn test_insert_update() {
    let map = CuckooHashMap::new();

    // Insert initial value
    map.insert(1, "one").unwrap();
    assert_eq!(map.get(&1), Some("one"));

    // Update with new value
    let old_value = map.insert(1, "ONE").unwrap();
    assert_eq!(old_value, Some("one")); // Returns old value
    assert_eq!(map.get(&1), Some("ONE")); // New value stored
    assert_eq!(map.len(), 1); // Size unchanged
}

#[test]
fn test_contains_key() {
    let map = CuckooHashMap::new();

    map.insert("key1", 100).unwrap();

    assert!(map.contains_key(&"key1"));
    assert!(!map.contains_key(&"key2"));
}

#[test]
fn test_get_key_value() {
    let map = CuckooHashMap::new();

    map.insert("hello", 42).unwrap();

    let result = map.get_key_value(&"hello");
    assert_eq!(result, Some(("hello", 42)));

    let result = map.get_key_value(&"world");
    assert_eq!(result, None);
}

#[test]
fn test_remove() {
    let map = CuckooHashMap::new();

    map.insert(1, "one").unwrap();
    map.insert(2, "two").unwrap();
    map.insert(3, "three").unwrap();

    assert_eq!(map.len(), 3);

    // Remove existing key
    let removed = map.remove(&2);
    assert_eq!(removed, Some("two"));
    assert_eq!(map.len(), 2);
    assert!(!map.contains_key(&2));

    // Remove non-existent key
    let removed = map.remove(&99);
    assert_eq!(removed, None);
    assert_eq!(map.len(), 2);

    // Verify other keys still exist
    assert_eq!(map.get(&1), Some("one"));
    assert_eq!(map.get(&3), Some("three"));
}

#[test]
fn test_clear() {
    let map = CuckooHashMap::new();

    // Insert some values
    for i in 0..50 {
        map.insert(i, i * 2).unwrap();
    }

    assert_eq!(map.len(), 50);
    assert!(!map.is_empty());

    // Clear the map
    map.clear();

    assert_eq!(map.len(), 0);
    assert!(map.is_empty());

    // Verify all keys are gone
    for i in 0..50 {
        assert_eq!(map.get(&i), None);
    }

    // Can still insert after clear
    map.insert(100, 200).unwrap();
    assert_eq!(map.get(&100), Some(200));
}

#[test]
fn test_capacity_and_load_factor() {
    let map: CuckooHashMap<i32, i32> = CuckooHashMap::new();

    let initial_capacity = map.capacity();
    assert!(initial_capacity > 0);

    // Insert elements
    for i in 0..10 {
        map.insert(i, i).unwrap();
    }

    let load_factor = map.load_factor();
    assert!(load_factor >= 0.0 && load_factor <= 1.0);

    let expected_lf = 10.0 / initial_capacity as f64;
    assert!((load_factor - expected_lf).abs() < 0.01);
}

#[test]
fn test_with_capacity() {
    let map: CuckooHashMap<i32, i32> = CuckooHashMap::with_capacity(1000);

    assert!(map.capacity() >= 1000);
    assert_eq!(map.len(), 0);
}

#[test]
fn test_iter() {
    let map = CuckooHashMap::new();

    // Insert test data
    map.insert(1, "one").unwrap();
    map.insert(2, "two").unwrap();
    map.insert(3, "three").unwrap();

    // Collect all entries
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by_key(|(k, _)| *k);

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0], (1, "one"));
    assert_eq!(entries[1], (2, "two"));
    assert_eq!(entries[2], (3, "three"));
}

#[test]
fn test_keys() {
    let map = CuckooHashMap::new();

    map.insert(10, "a").unwrap();
    map.insert(20, "b").unwrap();
    map.insert(30, "c").unwrap();

    let mut keys: Vec<_> = map.keys().collect();
    keys.sort();

    assert_eq!(keys, vec![10, 20, 30]);
}

#[test]
fn test_values() {
    let map = CuckooHashMap::new();

    map.insert(1, 100).unwrap();
    map.insert(2, 200).unwrap();
    map.insert(3, 300).unwrap();

    let mut values: Vec<_> = map.values().collect();
    values.sort();

    assert_eq!(values, vec![100, 200, 300]);
}

#[test]
fn test_into_iter() {
    let map = CuckooHashMap::new();

    map.insert("a", 1).unwrap();
    map.insert("b", 2).unwrap();
    map.insert("c", 3).unwrap();

    let mut entries: Vec<_> = map.into_iter().collect();
    entries.sort_by_key(|(k, _)| *k);

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0], ("a", 1));
    assert_eq!(entries[1], ("b", 2));
    assert_eq!(entries[2], ("c", 3));
}

#[test]
fn test_from_iter() {
    let data = vec![(1, "one"), (2, "two"), (3, "three")];
    let map: CuckooHashMap<_, _> = data.into_iter().collect();

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&1), Some("one"));
    assert_eq!(map.get(&2), Some("two"));
    assert_eq!(map.get(&3), Some("three"));
}

#[test]
fn test_extend() {
    let mut map = CuckooHashMap::new();
    map.insert(1, "one").unwrap();

    let additional = vec![(2, "two"), (3, "three"), (4, "four")];
    map.extend(additional);

    assert_eq!(map.len(), 4);
    assert_eq!(map.get(&2), Some("two"));
    assert_eq!(map.get(&3), Some("three"));
    assert_eq!(map.get(&4), Some("four"));
}

#[test]
fn test_clone() {
    let map1 = CuckooHashMap::new();
    map1.insert(1, "one").unwrap();
    map1.insert(2, "two").unwrap();

    let map2 = map1.clone();

    assert_eq!(map2.len(), 2);
    assert_eq!(map2.get(&1), Some("one"));
    assert_eq!(map2.get(&2), Some("two"));

    // Verify independence
    map1.insert(3, "three").unwrap();
    assert_eq!(map1.len(), 3);
    assert_eq!(map2.len(), 2);
    assert!(!map2.contains_key(&3));
}

#[test]
fn test_string_keys() {
    let map = CuckooHashMap::new();

    map.insert("hello".to_string(), 1).unwrap();
    map.insert("world".to_string(), 2).unwrap();

    assert_eq!(map.get(&"hello".to_string()), Some(1));
    assert_eq!(map.get(&"world".to_string()), Some(2));
}

#[test]
fn test_complex_values() {
    #[derive(Debug, Clone, PartialEq)]
    struct Data {
        id: i32,
        name: String,
    }

    let map = CuckooHashMap::new();

    let data1 = Data { id: 1, name: "Alice".to_string() };
    let data2 = Data { id: 2, name: "Bob".to_string() };

    map.insert(1, data1.clone()).unwrap();
    map.insert(2, data2.clone()).unwrap();

    assert_eq!(map.get(&1), Some(data1));
    assert_eq!(map.get(&2), Some(data2));
}

#[test]
fn test_large_dataset() {
    let map = CuckooHashMap::new();
    let n = 10_000;

    // Insert large number of elements
    for i in 0..n {
        map.insert(i, i * 2).unwrap();
    }

    assert_eq!(map.len(), n);

    // Verify all elements
    for i in 0..n {
        assert_eq!(map.get(&i), Some(i * 2));
    }

    // Remove half
    for i in (0..n).step_by(2) {
        map.remove(&i);
    }

    assert_eq!(map.len(), n / 2);

    // Verify removed elements are gone
    for i in (0..n).step_by(2) {
        assert_eq!(map.get(&i), None);
    }

    // Verify remaining elements
    for i in (1..n).step_by(2) {
        assert_eq!(map.get(&i), Some(i * 2));
    }
}

#[test]
fn test_minimum_load_factor() {
    let map: CuckooHashMap<i32, i32> = CuckooHashMap::new();

    let initial_mlf = map.minimum_load_factor();
    assert!(initial_mlf > 0.0);

    map.set_minimum_load_factor(0.3);
    assert!((map.minimum_load_factor() - 0.3).abs() < 0.001);
}

#[test]
fn test_maximum_hashpower() {
    let map: CuckooHashMap<i32, i32> = CuckooHashMap::new();

    let initial_mhp = map.maximum_hashpower();
    assert!(initial_mhp > 0);

    map.set_maximum_hashpower(25);
    assert_eq!(map.maximum_hashpower(), 25);
}

#[test]
fn test_hashpower() {
    let map: CuckooHashMap<i32, i32> = CuckooHashMap::new();

    let hp = map.hashpower();
    assert!(hp > 0);

    // Capacity should be 2^hashpower * SLOT_PER_BUCKET * 2
    let expected_buckets = 1 << hp;
    println!("Hashpower: {}, Expected buckets: {}", hp, expected_buckets);
}

#[test]
fn test_sequential_insert_remove() {
    let map = CuckooHashMap::new();

    for i in 0..100 {
        map.insert(i, i).unwrap();
    }

    for i in 0..100 {
        let val = map.remove(&i);
        assert_eq!(val, Some(i));
    }

    assert!(map.is_empty());
}

#[test]
fn test_overwrite_sequence() {
    let map = CuckooHashMap::new();

    // Insert and overwrite same key multiple times
    for version in 0..10 {
        let old = map.insert(42, version).unwrap();
        if version > 0 {
            assert_eq!(old, Some(version - 1));
        }
    }

    assert_eq!(map.get(&42), Some(9));
    assert_eq!(map.len(), 1);
}

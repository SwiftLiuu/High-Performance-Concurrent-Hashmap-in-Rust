/// Demo: Basic Operations of CuckooHashMap
/// Demonstrates core functionality

use cuckoo_hashmap::CuckooHashMap;

fn main() {
    println!("=== CuckooHashMap Basic Operations Demo ===\n");

    // 1. Create and insert
    println!("1. Creating map and inserting items:");
    let map = CuckooHashMap::with_capacity(8);
    map.insert("apple", 100).unwrap();
    map.insert("banana", 200).unwrap();
    map.insert("cherry", 300).unwrap();
    println!("   ✓ Inserted 3 items");
    println!("   - Length: {}\n", map.len());

    // 2. Retrieve values
    println!("2. Retrieving values:");
    println!("   - apple = {:?}", map.get(&"apple"));
    println!("   - banana = {:?}", map.get(&"banana"));
    println!("   - grape = {:?}\n", map.get(&"grape"));

    // 3. Update and remove
    println!("3. Update and remove:");
    let old = map.insert("apple", 150).unwrap();
    println!("   ✓ Updated apple: {:?} -> 150", old);
    let removed = map.remove(&"cherry");
    println!("   ✓ Removed cherry: {:?}", removed);
    println!("   - Length after removal: {}\n", map.len());

    // 4. Iteration
    println!("4. Iterating over entries:");
    for (key, value) in map.iter() {
        println!("   - {} = {}", key, value);
    }
    println!();

    // 5. Bulk insert
    println!("5. Bulk insert:");
    let map2: CuckooHashMap<i32, i32> = CuckooHashMap::with_capacity(8);
    for i in 1..=100 {
        map2.insert(i, i * 10).unwrap();
    }
    println!("   ✓ Inserted 100 items");
    println!("   - Total length: {}", map2.len());
    println!("   - Capacity: {}", map2.capacity());
    println!("   - Load factor: {:.2}%\n", map2.load_factor() * 100.0);

    // 6. Custom types
    println!("6. Working with custom types:");

    #[derive(Hash, Eq, PartialEq, Clone, Debug)]
    struct UserId(u32);

    #[derive(Clone, Debug)]
    struct User {
        name: String,
    }

    let users: CuckooHashMap<UserId, User> = CuckooHashMap::with_capacity(8);
    users.insert(UserId(1), User { name: "Alice".to_string() }).unwrap();
    users.insert(UserId(2), User { name: "Bob".to_string() }).unwrap();

    println!("   ✓ Inserted 2 users");
    if let Some(user) = users.get(&UserId(1)) {
        println!("   - User 1: {}", user.name);
    }

    println!("\n=== Demo Complete ===");
}

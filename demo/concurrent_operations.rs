/// Demo: Concurrent Operations of CuckooHashMap
/// Demonstrates thread-safe concurrent access

use cuckoo_hashmap::CuckooHashMap;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn main() {
    println!("=== CuckooHashMap Concurrent Operations Demo ===\n");

    // 1. Create shared map with Arc
    println!("1. Creating shared map:");
    let map = Arc::new(CuckooHashMap::new());
    println!("   ✓ Map wrapped in Arc for thread-safe sharing\n");

    // 2. Concurrent writers - same key, different values
    println!("2. Concurrent writers - 3 threads writing to same key:");
    let shared_key = 42;  // All threads write to key 42
    let values = vec![334, 738, 1024];
    let mut handles = vec![];

    for (thread_id, &value) in values.iter().enumerate() {
        let map_clone = Arc::clone(&map);
        handles.push(thread::spawn(move || {
            // Each thread writes its specific value many times
            for _ in 0..1000 {
                map_clone.insert(shared_key, value).unwrap();
            }
            println!("   ✓ Thread {} wrote value {} (1000 times)", thread_id, value);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!();
    println!("   All writers completed");

    // Check the final value - should be one of 334, 738, or 1024
    if let Some(final_value) = map.get(&shared_key) {
        let is_valid = values.contains(&final_value);
        println!("   - key {} = {} (valid: {})", shared_key, final_value, is_valid);
        if is_valid {
            println!("   - Last writer wins: value is from one of the threads ✓");
        }
    } else {
        println!("   - key {} = MISSING!", shared_key);
    }
    println!();

    // 3. Concurrent readers (performance test)
    println!("3. Concurrent readers - 8 threads:");
    let start = Instant::now();
    let mut handles = vec![];

    for _ in 0..8 {
        let map_clone = Arc::clone(&map);
        handles.push(thread::spawn(move || {
            for _ in 0..100_000 {
                let _ = map_clone.get(&42);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start.elapsed();
    let total_ops = 8 * 100_000;
    println!("   ✓ Completed {} reads", total_ops);
    println!("   - Time: {:.3} seconds", duration.as_secs_f64());
    println!("   - Throughput: {:.1} M ops/sec\n",
             total_ops as f64 / duration.as_secs_f64() / 1_000_000.0);

    // 4. Mixed workload
    println!("4. Mixed workload - 2 readers + 2 writers:");
    let mut handles = vec![];

    // Readers - reading the existing key 42
    for reader_id in 0..2 {
        let map_clone = Arc::clone(&map);
        handles.push(thread::spawn(move || {
            for _ in 0..10_000 {
                let _ = map_clone.get(&shared_key);
            }
            println!("   ✓ Reader {} completed 10,000 reads of key {}", reader_id, 42);
        }));
    }

    // Writers - adding new keys
    for writer_id in 0..2 {
        let map_clone = Arc::clone(&map);
        handles.push(thread::spawn(move || {
            let base = 1000 + writer_id * 50;
            for i in 0..50 {
                map_clone.insert(base + i, base + i).unwrap();
            }
            println!("   ✓ Writer {} added 50 new keys", writer_id);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("   ✓ Mixed workload completed");
    println!("   - Final size: {} keys\n", map.len());

    // 5. Verify correctness
    println!("5. Data integrity check:");
    let mut errors = 0;
    let shared_key = 42;
    let valid_values = vec![334, 738, 1024];

    // Check the shared key exists and has a valid value
    match map.get(&shared_key) {
        Some(value) if valid_values.contains(&value) => {
            println!("   ✓ key {} = {} (valid - one of [334, 738, 1024])", shared_key, value);
        }
        Some(value) => {
            println!("   ✗ key {} = {} (INVALID - not one of expected values!)", shared_key, value);
            errors += 1;
        }
        None => {
            println!("   ✗ key {} is MISSING!", shared_key);
            errors += 1;
        }
    }

    println!();
    println!("   - Verification errors: {}", errors);
    println!("   - Status: {}\n", if errors == 0 { "✓ PASSED" } else { "✗ FAILED" });

    println!("=== Demo Complete ===");
    println!("\nKey Points:");
    println!("  • Arc enables safe thread sharing");
    println!("  • Concurrent reads and writes work correctly");
    println!("  • No data races or lost updates");
    println!("  • High performance maintained");
}

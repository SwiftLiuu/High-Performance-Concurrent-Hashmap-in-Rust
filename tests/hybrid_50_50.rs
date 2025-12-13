use cuckoo_hashmap::CuckooHashMap;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

const PRELOAD: usize = 100_000;
const DURATION: Duration = Duration::from_secs(5);
const THREAD_COUNTS: &[usize] = &[1, 2, 4, 8];

fn run_workload(name: &str, insert_pct: u32) {
    for &threads in THREAD_COUNTS {
        let map = Arc::new(CuckooHashMap::new());

        // Preload to give reads something to find.
        for key in 0..PRELOAD {
            let _ = map.insert(key, key);
        }

        let barrier = Arc::new(Barrier::new(threads));
        let total_ops = Arc::new(AtomicU64::new(0));

        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let map = Arc::clone(&map);
                let barrier = Arc::clone(&barrier);
                let total_ops = Arc::clone(&total_ops);

                thread::spawn(move || {
                    let mut rng = rand::thread_rng();
                    barrier.wait();
                    let start = Instant::now();
                    let deadline = start + DURATION;
                    let mut local = 0u64;

                    while Instant::now() < deadline {
                        if rng.gen_range(0..100) < insert_pct {
                            let key = rng.gen_range(0..(PRELOAD * 2)) as usize;
                            let _ = map.insert(key, key);
                        } else {
                            let key = rng.gen_range(0..(PRELOAD * 2)) as usize;
                            let _ = map.get(&key);
                        }
                        local += 1;
                    }

                    total_ops.fetch_add(local, Ordering::Relaxed);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let throughput = total_ops.load(Ordering::Relaxed) as f64 / DURATION.as_secs_f64() / 1_000_000.0;
        println!("{name}: threads={threads} → throughput = {:.2} M ops/s", throughput);
    }
}

#[test]
fn hybrid_50_50() {
    run_workload("hybrid-50-50", 50);
    assert!(true);
}

//! Basic performance benchmarks for xTranslator
//!
//! Generic benchmarks that measure Rust standard library performance
//! characteristics relevant to xTranslator's string-heavy workloads.
//! These tests have NO dependency on xt-core or Skyrim data files.
//!
//! Run: cargo test --release --test basic_benchmarks

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_strings(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("Test string {}", i + 1)).collect()
}

fn make_regex_strings(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("Test string {} with dragon and sword enhanced", i))
        .collect()
}

// ---------------------------------------------------------------------------
// 1. String operations (filter, sort, search, HashMap insert)
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_string_operations() {
    println!("─── String Operations ───");

    let strings = make_strings(100_000);

    // filter
    let start = Instant::now();
    let _filtered: Vec<_> = strings.iter().filter(|s| s.contains("dragon")).collect();
    let t = start.elapsed();
    println!("  Filter 'dragon' from 100K  : {:?}", t);
    assert!(t < Duration::from_millis(100), "filter too slow: {:?}", t);

    // sort
    let start = Instant::now();
    let mut sorted = strings.clone();
    sorted.sort();
    let t = start.elapsed();
    println!("  Sort 100K strings          : {:?}", t);
    assert!(t < Duration::from_millis(500), "sort too slow: {:?}", t);

    // search (case-insensitive take)
    let start = Instant::now();
    let _found: Vec<_> = strings
        .iter()
        .filter(|s| s.to_lowercase().contains("test"))
        .take(1000)
        .collect();
    let t = start.elapsed();
    println!("  Case-insensitive search    : {:?}", t);
    assert!(t < Duration::from_millis(50), "search too slow: {:?}", t);

    // HashMap insert
    let start = Instant::now();
    let mut map = HashMap::new();
    for (i, s) in strings.iter().enumerate() {
        map.insert(i, s.clone());
    }
    let t = start.elapsed();
    println!("  HashMap insert 100K        : {:?}", t);
    assert!(t < Duration::from_millis(200), "HashMap too slow: {:?}", t);
}

// ---------------------------------------------------------------------------
// 2. Memory allocation patterns
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_memory_patterns() {
    println!("─── Memory Allocation Patterns ───");

    // Vec with pre-allocated capacity
    let start = Instant::now();
    let mut vec = Vec::with_capacity(100_000);
    for i in 0..100_000 {
        vec.push(format!("Memory test {}", i));
    }
    let t = start.elapsed();
    println!("  Vec with capacity 100K     : {:?}", t);
    assert!(t < Duration::from_millis(20), "Vec alloc too slow: {:?}", t);

    // String concatenation
    let start = Instant::now();
    let mut result = String::new();
    for i in 0..10_000 {
        result.push_str(&format!("Concat {}", i));
    }
    let t = start.elapsed();
    println!("  String concat 10K           : {:?}", t);
    assert!(t < Duration::from_millis(50), "concat too slow: {:?}", t);

    // HashMap with pre-allocated capacity
    let start = Instant::now();
    let mut map = HashMap::with_capacity(50_000);
    for i in 0..50_000 {
        map.insert(i, format!("Value {}", i));
    }
    let t = start.elapsed();
    println!("  HashMap with capacity 50K   : {:?}", t);
    assert!(
        t < Duration::from_millis(100),
        "HashMap alloc too slow: {:?}",
        t
    );
}

// ---------------------------------------------------------------------------
// 3. Regex operations
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_regex_operations() {
    println!("─── Regex Operations ───");

    let strings = make_regex_strings(10_000);

    // Simple regex
    let start = Instant::now();
    let re = regex::Regex::new(r"dragon").unwrap();
    let _n: usize = strings.iter().filter(|s| re.is_match(s)).count();
    let t = start.elapsed();
    println!("  Simple regex 10K            : {:?}", t);
    assert!(
        t < Duration::from_millis(200),
        "simple regex too slow: {:?}",
        t
    );

    // Complex regex
    let start = Instant::now();
    let re = regex::Regex::new(r"(dragon|sword).*\d+").unwrap();
    let _n: usize = strings.iter().filter(|s| re.is_match(s)).count();
    let t = start.elapsed();
    println!("  Complex regex 10K           : {:?}", t);
    assert!(
        t < Duration::from_millis(300),
        "complex regex too slow: {:?}",
        t
    );

    // Multiple regex patterns
    let start = Instant::now();
    let patterns: Vec<regex::Regex> = vec![
        regex::Regex::new(r"dragon").unwrap(),
        regex::Regex::new(r"sword").unwrap(),
        regex::Regex::new(r"test").unwrap(),
    ];
    let _n: usize = strings
        .iter()
        .filter(|s| patterns.iter().any(|p| p.is_match(s)))
        .count();
    let t = start.elapsed();
    println!("  Multi-pattern regex 10K     : {:?}", t);
    assert!(
        t < Duration::from_millis(400),
        "multi regex too slow: {:?}",
        t
    );
}

// ---------------------------------------------------------------------------
// 4. File I/O simulation
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_file_io() {
    println!("─── File I/O Simulation ───");

    let mut content = String::new();
    for i in 0..50_000 {
        content.push_str(&format!("Line {}: Some test content\n", i));
    }

    let file_path = std::env::temp_dir().join("xt-benchmark-io.txt");

    // Write
    let start = Instant::now();
    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }
    let t_write = start.elapsed();
    let size_kb = content.len() / 1024;
    println!("  Write ~{}KB file             : {:?}", size_kb, t_write);
    assert!(
        t_write < Duration::from_secs(1),
        "write too slow: {:?}",
        t_write
    );

    // Read
    let start = Instant::now();
    let _read_back = std::fs::read_to_string(&file_path).unwrap();
    let t_read = start.elapsed();
    println!("  Read back                   : {:?}", t_read);
    assert!(
        t_read < Duration::from_millis(500),
        "read too slow: {:?}",
        t_read
    );

    let _ = std::fs::remove_file(&file_path);
}

// ---------------------------------------------------------------------------
// 5. JSON serialization
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_json_serialization() {
    println!("─── JSON Serialization ───");

    let data: Vec<serde_json::Value> = (0..10_000)
        .map(|i| {
            serde_json::json!({
                "id": i,
                "source": format!("Source string {}", i),
                "translation": format!("Translation {}", i),
                "record_type": "INFO",
                "field_type": "FULL",
            })
        })
        .collect();

    // Serialize
    let start = Instant::now();
    let json_string = serde_json::to_string(&data).unwrap();
    let t = start.elapsed();
    println!("  Serialize 10K objects       : {:?}", t);
    assert!(
        t < Duration::from_millis(500),
        "serialize too slow: {:?}",
        t
    );

    // Deserialize
    let start = Instant::now();
    let _restored: Vec<serde_json::Value> = serde_json::from_str(&json_string).unwrap();
    let t = start.elapsed();
    println!("  Deserialize 10K objects     : {:?}", t);
    assert!(
        t < Duration::from_millis(300),
        "deserialize too slow: {:?}",
        t
    );
}

// ---------------------------------------------------------------------------
// 6. Concurrent operations
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_concurrent_operations() {
    println!("─── Concurrent Operations ───");

    let data = Arc::new(
        (0..50_000)
            .map(|i| format!("Concurrent test string {}", i))
            .collect::<Vec<_>>(),
    );

    let start = Instant::now();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let data = Arc::clone(&data);
            thread::spawn(move || data.iter().filter(|s| s.contains("test")).count())
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
    let t = start.elapsed();
    println!("  4 threads filter 50K        : {:?}", t);
    assert!(
        t < Duration::from_millis(200),
        "concurrent too slow: {:?}",
        t
    );
}

// ---------------------------------------------------------------------------
// 7. Scaling analysis
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_scaling() {
    println!("─── Scaling Analysis ───");

    let sizes = [1_000usize, 10_000, 100_000];
    let mut timings = Vec::new();

    for &size in &sizes {
        let start = Instant::now();
        let data = make_strings(size);
        let _sorted = {
            let mut c = data.clone();
            c.sort();
            c
        };
        timings.push((size, start.elapsed()));
    }

    for (size, dur) in &timings {
        println!("  Size {:>6}  : {:?}", size, dur);
    }

    // Check scaling ratios
    if timings.len() >= 3 {
        let r1 = timings[1].1.as_secs_f64() / timings[0].1.as_secs_f64();
        let r2 = timings[2].1.as_secs_f64() / timings[1].1.as_secs_f64();
        println!("  10K/1K ratio : {:.2}x", r1);
        println!("  100K/10K    : {:.2}x", r2);
        assert!(r1 < 18.0, "poor 1K→10K scaling: {:.2}x", r1);
        assert!(r2 < 18.0, "poor 10K→100K scaling: {:.2}x", r2);
    }
}

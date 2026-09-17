//! CodSpeed proof-of-concept: benchmarks the `Yadf::scan` hot path.
//!
//! Builds a small, deterministic file tree once per size under
//! `target/codspeed-bench/`, then measures duplicate detection over it.
//! Sizes are kept in the hundreds/low-thousands: under CodSpeed's
//! instruction-level simulation this is orders of magnitude slower than
//! wall-clock, unlike the 150k-file corpus `scripts/bench.sh` uses.

use std::fs;
use std::path::PathBuf;

fn main() {
    divan::main();
}

/// (Re)builds `file_count` files under a fresh directory: half share one
/// payload (duplicates), half are unique, so `scan` has real matching work.
fn build_corpus(file_count: usize) -> PathBuf {
    let dir = PathBuf::from("target/codspeed-bench").join(file_count.to_string());
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("failed to create corpus directory");
    let payload = vec![0xABu8; 4096];
    for i in 0..file_count {
        let path = dir.join(format!("file-{i}"));
        if i % 2 == 0 {
            fs::write(&path, &payload).expect("failed to write duplicate file");
        } else {
            fs::write(&path, format!("unique content {i}")).expect("failed to write unique file");
        }
    }
    dir
}

#[divan::bench(args = [100, 500, 1000])]
fn scan(bencher: divan::Bencher, file_count: usize) {
    let dir = build_corpus(file_count);
    bencher.bench_local(|| {
        yadf::Yadf::builder()
            .paths(vec![dir.clone()])
            .build()
            .scan::<seahash::SeaHasher>()
    });
}

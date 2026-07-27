//! The thread pool the I/O-bound hashing phases run on, and the process
//! limits they need raised before they can use it.

use super::advise;

/// Default concurrency for the I/O-bound hashing phases, distinct from (but
/// currently equal to) the walker's concurrency.
///
/// Oversubscribing well past the core count can help small random reads on
/// some SSD/NVMe devices saturate their queue depth (see pkolaczk's
/// disk-parallelism measurements), but it is not a safe default: measured
/// locally, going from 1x to 4x cores bought a few percent on a cold cache
/// while costing up to 60% on a warm one, because the extra threads have
/// nothing but each other to contend with once there is no device latency
/// to hide. Use `--io-threads` to opt into oversubscription on storage
/// where it is known to help.
pub fn default_threads() -> usize {
    num_cpus::get()
}

/// Runs `work` on a pool of `threads` I/O workers.
///
/// The open-file limit is raised once per process on the way in: a pool this
/// wide, plus the walker's own, can otherwise run into a distro-default soft
/// `RLIMIT_NOFILE`.
pub fn install<T: Send>(threads: usize, work: impl FnOnce() -> T + Send) -> T {
    static RAISE_NOFILE_LIMIT: std::sync::Once = std::sync::Once::new();
    RAISE_NOFILE_LIMIT.call_once(advise::raise_nofile_limit);
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("failed to build I/O thread pool")
        .install(work)
}

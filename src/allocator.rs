//! mimalloc as the global allocator, tuned for yadf's allocation pattern.

// glibc malloc keeps freed memory in per-thread arenas: paths allocated by the
// walker threads and freed by the hashing pool leave most of them half empty.
// mimalloc is built with `no_thp`, as committing its arenas on transparent huge
// pages costs more system time than glibc malloc on short runs.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Index of `mi_option_page_reclaim_on_free` in mimalloc v3's `mi_option_t`,
/// which libmimalloc-sys does not export.
const MI_OPTION_PAGE_RECLAIM_ON_FREE: libmimalloc_sys::mi_option_t = 35;

/// Lets mimalloc reclaim a page on any thread that frees into it: by default
/// only the thread that allocated it may, and the walker threads exit while the
/// hashing pool still frees their paths. Must run before any thread is spawned,
/// and leaves `MIMALLOC_PAGE_RECLAIM_ON_FREE` the last word, as mimalloc has
/// already read its options from the environment by then.
#[allow(unsafe_code)]
pub fn setup() {
    if std::env::var_os("MIMALLOC_PAGE_RECLAIM_ON_FREE").is_none() {
        // SAFETY: mi_option_set is not thread safe, and no thread is spawned yet.
        unsafe { libmimalloc_sys::mi_option_set(MI_OPTION_PAGE_RECLAIM_ON_FREE, 1) };
    }
}

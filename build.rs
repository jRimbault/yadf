fn main() {
    // Make the current git hash available to the build.
    let maybe_rev = git_revision_hash();
    if let Some(rev) = maybe_rev.as_deref() {
        println!("cargo:rustc-env=YADF_BUILD_GIT_HASH={}", rev);
    }
    let long_version = long_version(maybe_rev);
    println!("cargo:rustc-env=YADF_BUILD_VERSION={}", long_version);
}

fn git_revision_hash() -> Option<String> {
    let result = std::process::Command::new("git")
        .args(&["rev-parse", "--short=10", "HEAD"])
        .output();
    result.ok().and_then(|output| {
        let v = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    })
}

fn long_version(rev: Option<String>) -> String {
    // Do we have a git hash?
    // (Yes, if ripgrep was built on a machine with `git` installed.)
    let hash = match rev {
        None => String::new(),
        Some(githash) => format!(" (rev {})", githash),
    };
    let runtime = runtime_cpu_features();
    if runtime.is_empty() {
        format!(
            "{}{}|{} (compiled)",
            env!("CARGO_PKG_VERSION"),
            hash,
            compile_cpu_features().join(" ")
        )
    } else {
        format!(
            "{}{}|{} (compiled)|{} (runtime)",
            env!("CARGO_PKG_VERSION"),
            hash,
            compile_cpu_features().join(" "),
            runtime.join(" ")
        )
    }
}

/// Returns the relevant CPU features enabled at compile time.
fn compile_cpu_features() -> Vec<&'static str> {
    let mut features = vec![];
    if cfg!(feature = "avx-accel") {
        features.push("+AVX");
    } else {
        features.push("-AVX");
    }
    features
}

/// Returns the relevant CPU features enabled at runtime.
#[cfg(target_arch = "x86_64")]
fn runtime_cpu_features() -> Vec<&'static str> {
    let mut features = vec![];
    if is_x86_feature_detected!("avx2") {
        features.push("+AVX");
    } else {
        features.push("-AVX");
    }
    features
}

/// Returns the relevant CPU features enabled at runtime.
#[cfg(not(target_arch = "x86_64"))]
fn runtime_cpu_features() -> Vec<&'static str> {
    vec![]
}

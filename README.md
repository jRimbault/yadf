# Yet Another Dupes Finder

*It's [fast][benchmarks] on my machine.*

Installation:

```bash
cargo install yadf --features build-bin
```

CLI Usage:

```
yadf 0.1.0
Yet Another Dupes Finder

USAGE:
    yadf [FLAGS] [OPTIONS] [path]

FLAGS:
    -h, --help        Prints help information
    -n, --no-empty    exclude empty files
    -r, --report      print human readable report to stderr
    -V, --version     Prints version information

OPTIONS:
    -a, --algorithm <algorithm>    hashing algorithm [default: highway]
    -f, --format <format>          output format `json`, `json_pretty`, `fdupes` or `machine` [default:
                                   fdupes]
        --max <max>                maximum file size (default no maximum)
        --min <min>                minimum file size (default 0 byte)

ARGS:
    <path>    directory to search [default: .]
```

Library usage:

```rust
let path: &Path = "any/path".as_ref();
let duplicates_list = yadf::count_files::<twox_hash::XxHash64>(path, None);

print!("{}", duplicates_list.display::<yadf::Fdupes>());
serde_json::to_writer(std::io::stdout(), &duplicates_list).unwrap();
eprintln!("{}", yadf::Report::from(&duplicates_list));

for duplicates in files_counter.duplicates() {
    println!("There are {} instances of this file", duplicates.len());
}
```

Building (with or without `--release`):
- the library: `cargo build`.
- the executable: `cargo build --features build-bin`.

[benchmarks]: bench.md

# Yet Another Dupes Finder

*It's [fast][benchmarks] on my machine.*

Installation:

```bash
cargo install yadf
```

CLI Usage:

```
yadf 0.3.0
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
    -f, --format <format>          output format `standard`, `json`, `json_pretty`, `fdupes` or `machine` [default:
                                   fdupes]
        --max <max>                maximum file size (default no maximum)
        --min <min>                minimum file size (default 0 byte)

ARGS:
    <path>    directory to search [default: .]
```

[benchmarks]: bench.md

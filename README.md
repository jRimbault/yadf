# Yet Another Dupes Finder

_It's [fast][benchmarks] on my machine._

Installation:

```bash
cargo install yadf
```

CLI Usage:

```
yadf 0.3.1
Yet Another Dupes Finder

USAGE:
    yadf [FLAGS] [OPTIONS] [paths]...

FLAGS:
    -h, --help        Prints help information
    -n, --no-empty    Exclude empty files
    -r, --report      Prints human readable report to stderr
    -V, --version     Prints version information

OPTIONS:
    -a, --algorithm <algorithm>    hashing algorithm [default: xxhash]
    -f, --format <format>          output format [default: fdupes]
        --max <max>                maximum file size [default: no maximum]
        --min <min>                minimum file size [default: no minimum]

ARGS:
    <paths>...    Directories to search
```

[benchmarks]: bench.md

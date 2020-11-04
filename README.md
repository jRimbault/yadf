# Yet Another Dupes Finder

_It's [fast][benchmarks] on my machine._

Installation:

```bash
cargo install yadf
```

CLI Usage:

```
yadf 0.6.0
Yet Another Dupes Finder

USAGE:
    yadf [FLAGS] [OPTIONS] [paths]...

FLAGS:
    -h, --help        Prints help information
    -n, --no-empty    Excludes empty files
    -r, --report      Prints human readable report to stderr
    -V, --version     Prints version information

OPTIONS:
    -a, --algorithm <algorithm>    hashing algorithm [default: highway]
    -f, --format <format>          output format [default: fdupes]
        --max <size>               maximum file size [default: no maximum]
        --min <size>               minimum file size [default: no minimum]

ARGS:
    <paths>...    Directories to search

For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive)
```

[benchmarks]: bench.md

# Yet Another Dupes Finder

_It's [fast][benchmarks] on my machine._

Installation:

```bash
cargo install yadf
```

CLI Usage:

```
yadf 0.7.0
Yet Another Dupes Finder

USAGE:
    yadf [FLAGS] [OPTIONS] [paths]...

FLAGS:
    -h, --help        Prints help information
    -n, --no-empty    Excludes empty files
    -q, --quiet       Pass many times for less log output
    -r, --report      Prints human readable report to stderr
    -V, --version     Prints version information
    -v, --verbose     Pass many times for more log output

OPTIONS:
    -a, --algorithm <algorithm>    Hashing algorithm [default: Highway]  [possible values: Highway,
                                   SeaHash, XxHash]
    -f, --format <format>          Output format [default: Fdupes]  [possible values: Csv, Fdupes,
                                   Json, JsonPretty, Machine]
        --max <size>               Maximum file size
        --min <size>               Minimum file size
    -p, --pattern <glob>           Check files with a name matching a glob pattern, see:
                                   https://docs.rs/globset/0.4.6/globset/index.html#syntax
    -R, --regex <regex>            Check files with a name matching a Perl-style regex, see:
                                   https://docs.rs/regex/1.4.2/regex/index.html#syntax

ARGS:
    <paths>...    Directories to search

For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive).
```

[benchmarks]: bench.md

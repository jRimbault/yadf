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
    -a, --algorithm <algorithm>    hashing algorithm [default: highway]
    -f, --format <format>          output format [default: fdupes]
        --max <size>               maximum file size [default: no maximum]
        --min <size>               minimum file size [default: no minimum]
    -p, --pattern <glob>           check files with a name matching a glob pattern
    -R, --regex <regex>            check files with a name matching a PCRE regex

ARGS:
    <paths>...    Directories to search

For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive).
```

<details>
    <summary>Long help</summary>

```
yadf 0.7.0
Yet Another Dupes Finder

USAGE:
    yadf.exe [FLAGS] [OPTIONS] [paths]...

FLAGS:
    -h, --help
            Prints help information

    -n, --no-empty
            Excludes empty files

    -q, --quiet
            Pass many times for less log output

    -r, --report
            Prints human readable report to stderr

    -V, --version
            Prints version information

    -v, --verbose
            Pass many times for more log output

            By default, it'll only report errors. Passing `-v` one time also prints warnings, `-vv` enables info
            logging, `-vvv` debug, and `-vvvv` trace.

OPTIONS:
    -a, --algorithm <algorithm>
            hashing algorithm

            `highway`, `seahash`, or `xxhash` [default: XxHash]
    -f, --format <format>
            output format

            `json`, `json_pretty`, `fdupes`, or `machine` [default: Fdupes]
        --max <size>
            maximum file size [default: no maximum]

        --min <size>
            minimum file size [default: no minimum]

    -p, --pattern <glob>
            check files with a name matching a glob pattern

            see: https://docs.rs/globset/0.4.6/globset/index.html#syntax
    -R, --regex <regex>
            check files with a name matching a Perl-style regex

            see: https://docs.rs/regex/1.4.2/regex/index.html#syntax

ARGS:
    <paths>...
            Directories to search

            default is to search inside the current working directory

For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive).
```

</details>

[benchmarks]: bench.md

# YADF — Yet Another Dupes Finder

> _It's [fast](#benchmarks) on my machine._

## Installation

### Prebuilt Packages

Executable binaries for some platforms are available in the [releases](https://github.com/jRimbault/yadf/releases) section.

### Building from source

1. [Install Rust Toolchain](https://www.rust-lang.org/tools/install)
2. Run `cargo install yadf`

## Usage

`yadf` always descends automatically into subdirectories. I thought about that quite a lot, and didn't think of a _really_ good reason not to.

```bash
yadf # find duplicate files in current directory
yadf ~/Documents ~/Pictures # find duplicate files in two directories
```

### Filtering

```bash
yadf --min 100M # find duplicate files of at least 100 MB
yadf --max 100M # find duplicate files below 100 MB
yadf --pattern '*.jpg' # find duplicate jpg
yadf --regex '^g' # find duplicate starting with 'g'
```

<details>
  <summary>Help output.</summary>

```
yadf 0.8.1
Yet Another Dupes Finder

USAGE:
    yadf.exe [FLAGS] [OPTIONS] [paths]...

FLAGS:
    -h, --help        Prints help information
    -n, --no-empty    Excludes empty files
    -q, --quiet       Pass many times for less log output
    -r, --report      Prints human readable report to stderr
    -V, --version     Prints version information
    -v, --verbose     Pass many times for more log output

OPTIONS:
    -a, --algorithm <algorithm>    Hashing algorithm [default: XxHash]  [possible values: Highway, SeaHash, XxHash]
    -f, --format <format>          Output format [default: Fdupes]  [possible values: Csv, Fdupes, Json, JsonPretty,
                                   Machine]
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

</details>

## Benchmarks

The performance of `yadf` is heavily tied to the hardware, specifically the NVMe SSD. I recommend `fclones` as it has specific hardware heuristics. and in general more features.

My home directory contains about 615k paths and 32 GB of data, and is probably a pathological case of file duplication with all the node_modules, python virtual environments, rust target, etc.

| Program         | Version | Warm Mean time (s) | Cold Mean time (s) |
| :-------------- | ------: | -----------------: | -----------------: |
| yadf            |   0.8.1 |          **2.856** |             21.810 |
| [fclones][0]    |   0.8.0 |              3.627 |         **15.439** |
| [jdupes][1]     |  1.14.0 |             10.526 |            111.194 |
| [ddh][2]        |  0.11.3 |              8.221 |             21.948 |
| [fddf][3]       |   1.7.0 |              5.047 |             27.718 |
| [rmlint][4]     |   2.9.0 |             14.143 |             60.722 |
| [dupe-krill][5] |   1.4.4 |              8.072 |            112.815 |

The script used to benchmark can be read [here](bench.sh).

<details>
    <summary>Raw output of <code>hyperfine</code>.</summary>

Warm cache:

```
Benchmark #1: fclones --min-size 0 -R ~
  Time (mean ± σ):      3.627 s ±  0.043 s    [User: 15.379 s, System: 12.571 s]
  Range (min … max):    3.571 s …  3.726 s    10 runs

Benchmark #2: jdupes -z -r ~
  Time (mean ± σ):     10.526 s ±  0.031 s    [User: 5.367 s, System: 5.096 s]
  Range (min … max):   10.475 s … 10.567 s    10 runs

Benchmark #3: rmlint --hidden ~
  Time (mean ± σ):     14.143 s ±  0.049 s    [User: 38.964 s, System: 14.541 s]
  Range (min … max):   14.049 s … 14.233 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):      8.221 s ±  0.035 s    [User: 34.391 s, System: 26.450 s]
  Range (min … max):    8.145 s …  8.277 s    10 runs

Benchmark #5: dupe-krill -s -d ~
  Time (mean ± σ):      8.072 s ±  0.027 s    [User: 5.007 s, System: 3.028 s]
  Range (min … max):    8.040 s …  8.120 s    10 runs

Benchmark #6: fddf -m 0 ~
  Time (mean ± σ):      5.047 s ±  0.064 s    [User: 9.872 s, System: 12.816 s]
  Range (min … max):    4.936 s …  5.122 s    10 runs

Benchmark #7: yadf ~
  Time (mean ± σ):      2.856 s ±  0.009 s    [User: 9.834 s, System: 13.386 s]
  Range (min … max):    2.843 s …  2.873 s    10 runs

Summary
  'yadf ~' ran
    1.27 ± 0.02 times faster than 'fclones --min-size 0 -R ~'
    1.77 ± 0.02 times faster than 'fddf -m 0 ~'
    2.83 ± 0.01 times faster than 'dupe-krill -s -d ~'
    2.88 ± 0.02 times faster than 'ddh ~'
    3.69 ± 0.02 times faster than 'jdupes -z -r ~'
    4.95 ± 0.02 times faster than 'rmlint --hidden ~'
```

Cold cache:

```
Benchmark #1: fclones --min-size 0 -R ~
  Time (mean ± σ):     15.439 s ±  0.690 s    [User: 22.313 s, System: 34.814 s]
  Range (min … max):   14.715 s … 16.690 s    10 runs

Benchmark #2: jdupes -z -r ~
  Time (mean ± σ):     111.194 s ±  0.643 s    [User: 18.491 s, System: 27.820 s]
  Range (min … max):   110.394 s … 112.507 s    10 runs

Benchmark #3: rmlint --hidden ~
  Time (mean ± σ):     60.722 s ±  3.917 s    [User: 38.825 s, System: 24.832 s]
  Range (min … max):   57.520 s … 70.066 s    10 runs

Benchmark #4: ddh ~
  Time (mean ± σ):     21.948 s ±  1.138 s    [User: 39.015 s, System: 42.882 s]
  Range (min … max):   21.004 s … 24.579 s    10 runs

Benchmark #5: dupe-krill -s -d ~
  Time (mean ± σ):     112.815 s ±  0.621 s    [User: 20.133 s, System: 27.512 s]
  Range (min … max):   111.902 s … 113.747 s    10 runs

Benchmark #6: fddf -m 0 ~
  Time (mean ± σ):     27.718 s ±  0.526 s    [User: 18.505 s, System: 37.530 s]
  Range (min … max):   26.796 s … 28.407 s    10 runs

Benchmark #7: yadf ~
  Time (mean ± σ):     21.810 s ±  2.827 s    [User: 19.814 s, System: 53.879 s]
  Range (min … max):   20.054 s … 28.731 s    10 runs

  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet PC without any interferences from other programs. It might help to use the '--warmup' or '--prepare' options.

Summary
  'fclones --min-size 0 -R ~' ran
    1.41 ± 0.19 times faster than 'yadf ~'
    1.42 ± 0.10 times faster than 'ddh ~'
    1.80 ± 0.09 times faster than 'fddf -m 0 ~'
    3.93 ± 0.31 times faster than 'rmlint --hidden ~'
    7.20 ± 0.32 times faster than 'jdupes -z -r ~'
    7.31 ± 0.33 times faster than 'dupe-krill -s -d ~'
```

</details>

<details>
    <summary>Hardware used.</summary>

Extract from `neofetch` and `hwinfo --disk`:

- OS: Ubuntu 20.04.1 LTS x86_64
- Host: XPS 15 9570
- Kernel: 5.4.0-42-generic
- CPU: Intel i9-8950HK (12) @ 4.800GHz
- Memory: 4217MiB / 31755MiB
- Disk:
  - model: "SK hynix Disk"
  - driver: "nvme"

</details>

[0]: https://github.com/pkolaczk/fclones
[1]: https://github.com/jbruchon/jdupes
[2]: https://github.com/darakian/ddh
[3]: https://github.com/birkenfeld/fddf
[4]: https://github.com/sahib/rmlint
[5]: https://github.com/kornelski/dupe-krill

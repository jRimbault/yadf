# YADF — Yet Another Dupes Finder

> _It's [fast](#benchmarks) on my machine._

## Installation

### Prebuilt Packages

Executable binaries for some platforms are available in the [releases](https://github.com/jRimbault/yadf/releases) section.

### Building from source

1. [Install Rust Toolchain](https://www.rust-lang.org/tools/install)
2. Run `cargo install yadf`

## Usage

`yadf` defaults:

- search current working directory `$PWD`
- output format is the same as the "standard" `fdupes`, newline separated groups
- descends automatically into subdirectories
- search includes every files (including empty files)

```bash
yadf # find duplicate files in current directory
yadf ~/Documents ~/Pictures # find duplicate files in two directories
yadf --depth 0 file1 file2 # compare two files
yadf --depth 1 # find duplicates in current directory without descending
fd --type d a | yadf --depth 1 # find directories with an "a" and search them for duplicates without descending
fd --type f a | yadf # find files with an "a" and check them for duplicates
```

### Filtering

```bash
yadf --min 100M # find duplicate files of at least 100 MB
yadf --max 100M # find duplicate files below 100 MB
yadf --pattern '*.jpg' # find duplicate jpg
yadf --regex '^g' # find duplicate starting with 'g'
yadf --rfactor over:10 # find files with more than 10 copies
yadf --rfactor under:10 # find files with less than 10 copies
yadf --rfactor equal:1 # find unique files
```

### Formatting

Look up the help for a list of output formats `yadf -h`.

```bash
yadf -f json
yadf -f fdupes
yadf -f csv
yadf -f ldjson
```

<details>
  <summary>Help output.</summary>

```
yadf 0.13.1
Yet Another Dupes Finder

USAGE:
    yadf [FLAGS] [OPTIONS] [paths]...

FLAGS:
    -H, --hard-links    Treat hard links to same file as duplicates
    -h, --help          Prints help information
    -n, --no-empty      Excludes empty files
    -q, --quiet         Pass many times for less log output
    -V, --version       Prints version information
    -v, --verbose       Pass many times for more log output

OPTIONS:
    -a, --algorithm <algorithm>    Hashing algorithm [default: AHash]  [possible values: AHash,
                                   Highway, MetroHash, SeaHash, XxHash]
    -f, --format <format>          Output format [default: Fdupes]  [possible values: Csv, Fdupes,
                                   Json, JsonPretty, LdJson, Machine]
        --max <size>               Maximum file size
    -d, --depth <depth>            Maximum recursion depth
        --min <size>               Minimum file size
    -p, --pattern <glob>           Check files with a name matching a glob pattern, see:
                                   https://docs.rs/globset/0.4.6/globset/index.html#syntax
    -R, --regex <regex>            Check files with a name matching a Perl-style regex, see:
                                   https://docs.rs/regex/1.4.2/regex/index.html#syntax
        --rfactor <rfactor>        Replication factor [under|equal|over]:n

ARGS:
    <paths>...    Directories to search

For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive).
```

</details>

## Notes on the algorithm

Most¹ dupe finders follow a 3 steps algorithm:

1. group files by their size
2. group files by their first few bytes
3. group files by their entire content

`yadf` skips the first step, and only does the steps 2 and 3, preferring hashing rather than byte comparison. In my [tests][3-steps] having the first step on a SSD actually slowed down the program.
`yadf` makes heavy use of the standard library [`BTreeMap`][btreemap], it uses a cache aware implementation avoiding too many cache misses. `yadf` uses the parallel walker provided by `ignore` (disabling its _ignore_ features) and `rayon`'s parallel iterators to do each of these 2 steps in parallel.

¹: some need a different algorithm to support different features or different performance trade-offs

[btreemap]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
[3-steps]: https://github.com/jRimbault/yadf/tree/3-steps
[hashmap]: https://doc.rust-lang.org/std/collections/struct.HashMap.html

### Design goals

I sought out to build a high performing artefact by assembling together libraries doing the actual work, nothing here is custom made, it's all "off-the-shelf" software.

## Benchmarks

The performance of `yadf` is heavily tied to the hardware, specifically the
NVMe SSD. I recommend `fclones` as it has more hardware heuristics. and in general more features.

My home directory contains upwards of 700k paths and 39 GB of data, and is probably a pathological case of file duplication with all the node_modules, python virtual environments, rust target, etc. Arguably, the most important measure here is the mean time when the filesystem cache is cold.

| Program (warm filesystem cache) | Version |          Mean [s] |   Min [s] | Max [s] |    Relative |
| :------------------------------ | ------: | ----------------: | --------: | ------: | ----------: |
| [`fclones`][0]                  |   0.8.0 |     4.107 ± 0.045 |     4.065 |   4.189 | 1.58 ± 0.04 |
| [`jdupes`][1]                   |  1.14.0 |    11.982 ± 0.038 |    11.924 |  12.030 | 4.60 ± 0.11 |
| [`ddh`][2]                      |  0.11.3 |    10.602 ± 0.062 |    10.521 |  10.678 | 4.07 ± 0.10 |
| [`rmlint`][3]                   |   2.9.0 |    17.640 ± 0.119 |    17.426 |  17.833 | 6.77 ± 0.17 |
| [`dupe-krill`][4]               |   1.4.4 |     9.110 ± 0.040 |     9.053 |   9.154 | 3.50 ± 0.08 |
| [`fddf`][5]                     |   1.7.0 |     5.630 ± 0.049 |     5.562 |   5.717 | 2.16 ± 0.05 |
| `yadf`                          |  0.14.1 | **2.605 ± 0.062** |     2.517 |   2.676 |        1.00 |

| Program (cold filesystem cache) | Version |   Mean [s] |
| :------------------------------ | ------: | ---------: |
| [fclones][0]                    |   0.8.0 | **19.452** |
| [jdupes][1]                     |  1.14.0 |    129.132 |
| [ddh][2]                        |  0.11.3 |     27.241 |
| [rmlint][3]                     |   2.9.0 |     67.580 |
| [dupe-krill][4]                 |   1.4.4 |    127.860 |
| [fddf][5]                       |   1.7.0 |     32.661 |
| yadf                            |  0.13.1 |     21.554 |

`fdupes` is excluded from this benchmark because it's _really_ slow.

The script used to benchmark can be read [here](./bench.sh).

[0]: https://github.com/pkolaczk/fclones
[1]: https://github.com/jbruchon/jdupes
[2]: https://github.com/darakian/ddh
[3]: https://github.com/sahib/rmlint
[4]: https://github.com/kornelski/dupe-krill
[5]: https://github.com/birkenfeld/fddf

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

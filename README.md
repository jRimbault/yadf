# YADF — Yet Another Dupes Finder

> _It's [fast](#benchmarks) on my machine._

___

You should probably use [`fclones`][0].

___

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
NVMe SSD. I recommend `fclones` as it has more hardware heuristics. and in general more features. `yadf` on HDDs is _terrible_.

My home directory contains upwards of 700k paths and 39 GB of data, and is probably a pathological case of file duplication with all the node_modules, python virtual environments, rust target, etc. Arguably, the most important measure here is the mean time when the filesystem cache is cold.

| Program (warm filesystem cache) | Version |          Mean [s] |   Min [s] | Max [s] |
| :------------------------------ | ------: | ----------------: | --------: | ------: |
| [`fclones`][0]                  |  0.29.3 | 7.435 ± 1.609 | 4.622 | 9.317 |
| [`jdupes`][1]                   |  1.14.0 | 16.787 ± 0.208 | 16.484 | 17.178 |
| [`ddh`][2]                      |    0.13 | 12.703 ± 1.547 | 10.814 | 14.793 |
| [`dupe-krill`][4]               |   1.4.7 | 15.555 ± 1.633 | 12.486 | 16.959 |
| [`fddf`][5]                     |   1.7.0 | 18.441 ± 1.947 | 15.097 | 22.389 |
| `yadf`                          |   1.1.0 | **3.157 ± 0.638** | 2.362 | 4.175 |

| Program (cold filesystem cache) | Version |          Mean [s] |   Min [s] | Max [s] |
| :------------------------------ | ------: | ----------------: | --------: | ------: |
| [`fclones`][0]                  |  0.29.3 | 68.950 ± 3.694 | 63.165 | 73.534 |
| [`jdupes`][1]                   |  1.14.0 | 303.907 ± 11.578 | 277.618 | 314.226 |
| `yadf`                          |   1.1.0 | 52.481 ± 1.125 | 50.412 | 54.265 |

_I test less programs here because it takes several hours to run._

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

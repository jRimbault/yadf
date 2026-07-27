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
2. Run `cargo install --locked yadf`

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
Yet Another Dupes Finder

Usage: yadf [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Directories to search

Options:
  -f, --format <FORMAT>        Output format [default: fdupes] [possible values: csv, fdupes, json, json-pretty, ld-json, machine]
  -a, --algorithm <ALGORITHM>  Hashing algorithm [default: highway] [possible values: ahash, blake3, highway, metrohash, seahash, xxhash]
  -n, --no-empty               Excludes empty files
      --min <size>             Minimum file size
      --max <size>             Maximum file size
  -d, --depth <depth>          Maximum recursion depth
      --io-threads <n>         Concurrency for the I/O-bound hashing phases
  -H, --hard-links             Treat hard links to same file as duplicates
  -R, --regex <REGEX>          Check files with a name matching a Perl-style regex, see: https://docs.rs/regex/1.4.2/regex/index.html#syntax
  -p, --pattern <glob>         Check files with a name matching a glob pattern, see: https://docs.rs/globset/0.4.6/globset/index.html#syntax
  -v, --verbose...             Increase logging verbosity
  -q, --quiet...               Decrease logging verbosity
      --rfactor <RFACTOR>      Replication factor [under|equal|over]:n
  -o, --output <OUTPUT>        Optional output file
  -h, --help                   Print help (see more with '--help')
  -V, --version                Print version

For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive).
```

</details>

## Notes on the algorithm

Most¹ dupe finders follow a multi-step algorithm:

1. group files by their size
2. group files by their first few bytes
3. group files by their last few bytes (for large files)
4. group files by their entire content

Early versions of `yadf` skipped step 1, which was faster on a warm cache but meant reading
files that could never have matched anything. It now groups by size first and only opens a file
if another file shares its size. Step 3 is only done for files large enough to be worth it.
`yadf` makes heavy use of the standard library [`BTreeMap`][btreemap], it uses a cache aware implementation avoiding too many cache misses. `yadf` uses the parallel walker provided by `ignore` (disabling its _ignore_ features) and `rayon`'s parallel iterators to do each of these steps in parallel, with a separate, more concurrent thread pool (`--io-threads`) for the I/O-bound hashing steps.

On Linux a few extra threads `posix_fadvise` the files about to be read, keeping enough
requests in flight to hide device latency on a cold cache. They never hash, so unlike raising
`--io-threads` they cost almost nothing warm.

¹: some need a different algorithm to support different features or different performance trade-offs

[btreemap]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
[hashmap]: https://doc.rust-lang.org/std/collections/struct.HashMap.html

### Design goals

I sought out to build a high performing artefact by assembling together libraries doing the actual work, nothing here is custom made, it's all "off-the-shelf" software.

## Benchmarks

The performance of `yadf` is heavily tied to the hardware, specifically the
NVMe SSD. I recommend `fclones` as it has more hardware heuristics, and in general more features. `yadf` on HDDs is _terrible_.

The numbers below are from a reproducible synthetic corpus rather than a personal home
directory, so they can be regenerated exactly: `scripts/gen-corpus.py --seed 42 --files 150000
--dup-ratio 0.15 --collide-prefix 0.05 --size-dist realistic`, 150,001 files, 27.6 GB, 6,453
duplicate groups. Arguably, the most important measure here is the mean time when the
filesystem cache is cold.

| Program (warm filesystem cache) | Version | Mean [s]          | Min [s] | Max [s] |
| :------------------------------ | ------: | ----------------: | ------: | ------: |
| [`fclones`][0]                  |  0.34.0 |     0.676 ± 0.007 |   0.670 |   0.692 |
| [`jdupes`][1]                   |  1.20.2 |     2.649 ± 0.013 |   2.634 |   2.671 |
| [`ddh`][2]                      |  0.13.0 |     1.187 ± 0.022 |   1.159 |   1.220 |
| [`dupe-krill`][4]               |   1.5.0 |     4.031 ± 0.058 |   3.929 |   4.134 |
| [`fddf`][5]                     |   1.7.0 |     0.729 ± 0.013 |   0.708 |   0.749 |
| `yadf`                          |   1.4.0 | **0.594 ± 0.012** |   0.580 |   0.624 |

| Program (cold filesystem cache) | Version | Mean [s]           | Min [s] | Max [s] |
| :------------------------------ | ------: | -----------------: | ------: | ------: |
| [`fclones`][0]                  |  0.34.0 |      2.830 ± 0.015 |   2.808 |   2.843 |
| [`jdupes`][1]                   |  1.20.2 |     17.366 ± 0.043 |  17.332 |  17.435 |
| [`ddh`][2]                      |  0.13.0 |      3.160 ± 0.094 |   3.094 |   3.326 |
| [`dupe-krill`][4]               |   1.5.0 |     17.746 ± 0.037 |  17.701 |  17.783 |
| [`fddf`][5]                     |   1.7.0 |      2.990 ± 0.014 |   2.982 |   3.014 |
| `yadf`                          |   1.4.0 |  **2.804 ± 0.019** |   2.784 |   2.833 |

_Cold cache, the top three are within ~6%, and `yadf` and `fclones` are tied around
about one standard deviation apart._

`fclones group` skips empty files, hidden files, `.gitignore` matches and symlinks by default;
these runs pass `--min 0` and the corpus contains none of those. Benchmarking against a home
directory needs `--min 0 --hidden --no-ignore` to compare the same work.

The script used to benchmark against other tools can be read [here](./scripts/bench.sh). To
compare `yadf` against itself across commits or releases, see
[`scripts/bench-versions.sh`](./scripts/bench-versions.sh).

[0]: https://github.com/pkolaczk/fclones
[1]: https://github.com/jbruchon/jdupes
[2]: https://github.com/darakian/ddh
[3]: https://github.com/sahib/rmlint
[4]: https://github.com/kornelski/dupe-krill
[5]: https://github.com/birkenfeld/fddf

<details>
    <summary>Hardware used.</summary>

- OS: Ubuntu, kernel 6.8.0-124-generic
- CPU: 11th Gen Intel(R) Core(TM) i7-11850H @ 2.50GHz (16 threads)
- Memory: 15 GiB
- Disk: NVMe, CT2000P5PSSD8

</details>


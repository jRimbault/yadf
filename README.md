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

Early versions of `yadf` skipped step 1, hashing the file size together with the first bytes
instead: on a *warm* page cache, an extra directory pass to group by size was pure overhead. On
a *cold* cache that trade inverts — most files have a unique size and can be ruled out as
duplicates without ever being opened, so skipping step 1 means paying for a real disk read on
files that could never have matched anything. `yadf` groups by size first, and only opens a
file if at least one other file shares its size; only files that also collide on size *and* a
4 KiB prefix, and are large enough to be worth it, get a cheap 4 KiB tail read (step 3) before
the expensive full read (step 4).
`yadf` makes heavy use of the standard library [`BTreeMap`][btreemap], it uses a cache aware implementation avoiding too many cache misses. `yadf` uses the parallel walker provided by `ignore` (disabling its _ignore_ features) and `rayon`'s parallel iterators to do each of these steps in parallel, with a separate, more concurrent thread pool (`--io-threads`) for the I/O-bound hashing steps.

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
--dup-ratio 0.15 --collide-prefix 0.05 --size-dist realistic`, 150,001 files, 27.6 GB, with
6,453 duplicate groups and 3,750 near-duplicate (shared-prefix, differing content) pairs mixed
in. Each program was run with `hyperfine` against the same tree; see
[`scripts/bench.sh`](./scripts/bench.sh) for the exact commands. Arguably the most important
measure here is the mean time when the filesystem cache is cold, since that's the situation on
a first run.

| Program (warm filesystem cache) | Version | Mean [s]          | Min [s] | Max [s] |
| :------------------------------ | ------: | ----------------: | ------: | ------: |
| [`fclones`][0]                  |  0.34.0 |      1.107 ± 0.071 |   1.057 |   1.285 |
| [`jdupes`][1]                   |  1.20.2 |      3.041 ± 0.061 |   2.968 |   3.172 |
| [`ddh`][2]                      |  0.13.0 |      1.111 ± 0.026 |   1.089 |   1.166 |
| [`dupe-krill`][4]               |   1.5.0 |      4.044 ± 0.073 |   3.949 |   4.144 |
| [`fddf`][5]                     |   1.7.0 |      0.792 ± 0.016 |   0.779 |   0.822 |
| `yadf`                          |   1.3.0 | **0.603 ± 0.018** |   0.580 |   0.633 |

| Program (cold filesystem cache) | Version | Mean [s]           | Min [s] | Max [s] |
| :------------------------------ | ------: | -----------------: | ------: | ------: |
| [`fclones`][0]                  |  0.34.0 |  **2.856 ± 0.123** |   2.762 |   3.072 |
| [`jdupes`][1]                   |  1.20.2 |      17.553 ± 0.057 |  17.500 |  17.642 |
| [`ddh`][2]                      |  0.13.0 |       3.061 ± 0.061 |   2.976 |   3.139 |
| [`dupe-krill`][4]               |   1.5.0 |      17.904 ± 0.088 |  17.814 |  18.006 |
| [`fddf`][5]                     |   1.7.0 |       3.005 ± 0.025 |   2.975 |   3.037 |
| `yadf`                          |   1.3.0 |       3.348 ± 0.029 |   3.319 |   3.380 |

Warm cache is still where `yadf` is fastest here. On a cold cache, `fclones`, `ddh` and `fddf`
are now all within a few percent of each other, and `fclones` is slightly ahead of `yadf`, not
behind it as in earlier versions of this table. `fclones` in particular has clearly done real
work on its cold-cache path since 0.29.3.
This is on one machine with one corpus shape; the ranking among the top four is close enough
that it could plausibly flip on different hardware or a different file mix, which is exactly
why `scripts/bench-versions.sh` exists — to let anyone re-run this rather than trust a table.
`--io-threads` was swept from 16 to 128 on this run and made at most a ~5% difference, so the
default (one thread per core) was left as-is; it may matter more on other NVMe controllers.
`jdupes` and `dupe-krill` fall far behind on a cold cache specifically, suggesting a less
concurrent read path.

None of this changes the recommendation at the top of this document: `fclones` has more
hardware heuristics and more features, and is the safer default choice.

The script used to benchmark against other tools can be read [here](./scripts/bench.sh). To
compare `yadf` against itself across commits or releases, see
[`scripts/bench-versions.sh`](./scripts/bench-versions.sh), which builds each revision,
verifies they all report the same duplicates over a reproducible synthetic corpus (see
[`scripts/gen-corpus.py`](./scripts/gen-corpus.py)), then times them with `hyperfine`.

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


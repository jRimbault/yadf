# Benchmark scripts

Needs [`uv`](https://docs.astral.sh/uv/) and `hyperfine` >= 2.0. Run everything from this
directory. Generated corpora and built tools live in `../.bench-cache`, results in
`../bench-results/<timestamp>`.

## yadf against other duplicate finders

```sh
mise install                    # competitors at the versions pinned in mise.toml
mise exec -- ./bench.py         # warm cache
mise exec -- ./bench.py --cold  # cold cache, needs passwordless sudo to drop caches
```

`--install` instead `cargo install`s the latest published competitors. jdupes is always built
from source. The first run generates the default corpus (27.6 GB, see below).

## yadf against itself

```sh
./bench-versions.py main dirty            # "dirty" is the working tree as-is
./bench-versions.py --cold v1.4.4 HEAD
```

Each revision is built once and cached in `../.bench-cache/bin`.

Both scripts take `--corpus DIR`, `--warm-up N` and `--min-runs N`.

## Corpora

`gen-corpus.py` writes a deterministic tree: the same arguments always produce the same files.
The default corpus, used for the README numbers:

```sh
./gen-corpus.py --out ../.bench-cache/corpus --seed 42 --files 150000 \
    --dup-ratio 0.15 --collide-prefix 0.05 --size-dist realistic
```

Smaller or different shapes:

```sh
# 10k files, 1.4 GB: quick iterations
./gen-corpus.py --out ../.bench-cache/corpus-10k --seed 1 --files 10000 \
    --dup-ratio 0.15 --collide-prefix 0.05 --size-dist realistic
# 300k small files, 1.8 GB: metadata-heavy
./gen-corpus.py --out ../.bench-cache/corpus-300k --seed 3 --files 300000 \
    --dup-ratio 0.15 --collide-prefix 0.05 --size-dist small
# 1M small files in a deep tree, 5.9 GB
./gen-corpus.py --out ../.bench-cache/corpus-deep --seed 7 --files 1000000 \
    --dup-ratio 0.15 --collide-prefix 0.05 --size-dist small --fanout 4 --depth 8
```

Each corpus has a `manifest.json` with its parameters and expected duplicate counts.

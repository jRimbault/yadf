#!/usr/bin/env bash
#/ Usage: bench.sh [--cold] [--corpus DIR] [--install] [--min-runs N]
#/
#/ Benchmark yadf against the other duplicate finders over a reproducible
#/ synthetic corpus, and print a README-ready markdown table (program,
#/ version, mean/min/max). To compare yadf against itself across commits or
#/ releases, use scripts/bench-versions.sh instead.
#/
#/ Options:
#/   --cold          drop the page cache before each timed run (needs
#/                   passwordless sudo for `tee /proc/sys/vm/drop_caches`);
#/                   default is a warm-cache benchmark
#/   --corpus DIR    directory to scan; generated at .bench-cache/corpus with
#/                   scripts/gen-corpus.py on first use if not given
#/   --install       cargo install --locked the crates.io competitors at their
#/                   latest published version before benchmarking; jdupes is
#/                   always built from its latest source release into
#/                   .bench-cache/tools, since no usable binary is packaged
#/   --min-runs N    minimum hyperfine runs per program (default 10 warm, 5 cold)
#/   -h, --help      show this help
#/
#/ Every competitor is run so that it looks at the same files yadf does:
#/
#/   - fclones skips empty, hidden, gitignored and symlinked files by default,
#/     hence --min 0 --hidden --no-ignore
#/   - jdupes does not recurse by default, hence -r, and skips empty files
#/     without -z
#/   - dupe-krill skips files smaller than the block size (-s) and hardlinks
#/     matches together unless told not to (-d, dry run)
#/   - fddf ignores zero length files without -m 0
#/   - ddh writes a Results.txt in the working directory on every run
#/
#/ Note: .cargo/config.toml sets `-C target-cpu=native`, so results are only
#/ comparable across runs made on the SAME machine.
#/
#/ Examples:
#/   bench.sh
#/   bench.sh --install --cold
#/   bench.sh --corpus /data/corpus --min-runs 20

set -euo pipefail

usage() {
  grep "^#/" "$0" | cut -c4-
  exit "${1:-0}"
}

repo_root=$(git rev-parse --show-toplevel)
cache_dir="$repo_root/.bench-cache"
tools_dir="$cache_dir/tools"
timestamp=$(date +%Y%m%dT%H%M%S)
results_dir="$repo_root/bench-results/$timestamp"

# Corpus the README numbers are quoted against: deterministic given the seed.
corpus_files=150000
corpus_seed=42
corpus_dup_ratio=0.15
corpus_collide_prefix=0.05
corpus_size_dist=realistic

# Crates providing a competitor, installed with `cargo install --locked`.
cargo_competitors=(fclones ddh dupe-krill fddf)

cold=0
corpus=""
do_install=0
min_runs=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cold)
      cold=1
      shift
      ;;
    --corpus)
      corpus="$2"
      shift 2
      ;;
    --install)
      do_install=1
      shift
      ;;
    --min-runs)
      min_runs="$2"
      shift 2
      ;;
    -h | --help)
      usage 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage 1
      ;;
  esac
done

for tool in hyperfine cargo git python3 curl tar make cc; do
  command -v "$tool" >/dev/null || {
    echo "error: $tool is required but not found in PATH" >&2
    exit 1
  }
done

if [[ "$cold" -eq 1 ]] && ! sudo -n true 2>/dev/null; then
  echo "error: --cold requires passwordless sudo to drop the page cache" >&2
  exit 1
fi

mkdir -p "$tools_dir" "$results_dir"

# Latest published version of a crate, per the crates.io index.
crates_io_latest() {
  curl -sSf "https://crates.io/api/v1/crates/$1" -H 'User-Agent: yadf-bench' |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["crate"]["max_stable_version"])'
}

# Latest release tag of a codeberg repository, without its leading "v".
codeberg_latest() {
  curl -sSf "https://codeberg.org/api/v1/repos/jbruchon/$1/releases?limit=1" |
    python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["tag_name"].lstrip("v"))'
}

# Builds jdupes from the upstream source release into the tool cache.
#
# Distro packages lag several minor versions behind (Ubuntu 24.04 ships
# 1.20.2), and upstream's own prebuilt binaries hardcode an ELF interpreter
# path (/lib/ld-linux-x86-64.so.2) that does not exist on a usrmerge distro,
# so neither is usable here. jdupes needs libjodycode, which is not packaged
# either; its Makefile picks up a sibling ../libjodycode checkout and can link
# it statically, which is what this does.
build_jdupes() {
  local version="$1" ljc_version="$2" src_dir dest
  dest="$tools_dir/jdupes-$version/jdupes"
  if [[ -x "$dest" ]]; then
    echo "$dest"
    return
  fi
  src_dir="$tools_dir/src-jdupes-$version"
  echo "==> building jdupes $version against libjodycode $ljc_version" >&2
  rm -rf "$src_dir"
  mkdir -p "$src_dir"
  curl -sSfL "https://codeberg.org/jbruchon/libjodycode/archive/v$ljc_version.tar.gz" |
    tar -xzf - -C "$src_dir"
  curl -sSfL "https://codeberg.org/jbruchon/jdupes/archive/v$version.tar.gz" |
    tar -xzf - -C "$src_dir"
  make -C "$src_dir/libjodycode" -j"$(nproc)" >&2
  make -C "$src_dir/jdupes" -j"$(nproc)" static_jc >&2
  mkdir -p "$tools_dir/jdupes-$version"
  cp "$src_dir/jdupes/jdupes" "$dest"
  echo "$dest"
}

if [[ "$do_install" -eq 1 ]]; then
  for crate in "${cargo_competitors[@]}"; do
    echo "==> cargo install --locked $crate"
    cargo install --locked --quiet "$crate" || {
      echo "error: failed to install $crate" >&2
      exit 1
    }
  done
fi

jdupes_version=$(codeberg_latest jdupes)
libjodycode_version=$(codeberg_latest libjodycode)
jdupes_bin=$(build_jdupes "$jdupes_version" "$libjodycode_version")

# yadf is always benchmarked from the working tree, not from whatever happens
# to be installed in ~/.cargo/bin.
echo "==> building yadf from the working tree"
(cd "$repo_root" && cargo build --release --quiet)
yadf_bin="$repo_root/target/release/yadf"

if [[ -z "$corpus" ]]; then
  corpus="$cache_dir/corpus"
  if [[ ! -f "$corpus/manifest.json" ]]; then
    echo "==> generating default corpus at $corpus"
    python3 "$repo_root/scripts/gen-corpus.py" \
      --out "$corpus" --seed "$corpus_seed" --files "$corpus_files" \
      --dup-ratio "$corpus_dup_ratio" --collide-prefix "$corpus_collide_prefix" \
      --size-dist "$corpus_size_dist"
  fi
fi
if [[ ! -f "$corpus/manifest.json" ]]; then
  echo "error: corpus manifest not found at $corpus/manifest.json" >&2
  exit 1
fi

# Version strings, normalized to a bare number for the README table.
# dupe-krill has no --version; it prints "(vX.Y.Z)" in its help banner.
version_of() {
  local bin="$1" name="$2" out
  case "$name" in
    dupe-krill) out=$("$bin" --help 2>&1 | head -1) ;;
    *) out=$("$bin" --version 2>&1 | head -1) ;;
  esac
  echo "$out" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1
}

names=(fclones jdupes ddh dupe-krill fddf yadf)
declare -A binaries=(
  [fclones]="$(command -v fclones || true)"
  [jdupes]="$jdupes_bin"
  [ddh]="$(command -v ddh || true)"
  [dupe-krill]="$(command -v dupe-krill || true)"
  [fddf]="$(command -v fddf || true)"
  [yadf]="$yadf_bin"
)
declare -A arguments=(
  [fclones]="group --min 0 --hidden --no-ignore"
  [jdupes]="-z -r"
  [ddh]="--directories"
  [dupe-krill]="-s -d"
  [fddf]="-m 0"
  [yadf]=""
)

missing=()
for name in "${names[@]}"; do
  [[ -x "${binaries[$name]}" ]] || missing+=("$name")
done
if [[ ${#missing[@]} -gt 0 ]]; then
  echo "error: not found in PATH: ${missing[*]}" >&2
  echo "       run with --install to install the competitors" >&2
  exit 1
fi

declare -A versions=()
echo "==> versions under test"
for name in "${names[@]}"; do
  versions[$name]=$(version_of "${binaries[$name]}" "$name")
  printf '    %-12s %s\n' "$name" "${versions[$name]}"
done

# Competitors published on crates.io should be benchmarked at their latest
# release; warn rather than fail so an offline run still produces numbers.
for crate in "${cargo_competitors[@]}"; do
  latest=$(crates_io_latest "$crate" 2>/dev/null || true)
  if [[ -n "$latest" && "$latest" != "${versions[$crate]}" ]]; then
    echo "warning: $crate ${versions[$crate]} is installed, $latest is published" >&2
    echo "         re-run with --install to upgrade" >&2
  fi
done

# ddh drops a Results.txt in the working directory on every run; remove it
# between runs so no program is timed against a dirtied directory.
prepare_cmd="rm -f Results.txt"
if [[ "$cold" -eq 1 ]]; then
  prepare_cmd="$prepare_cmd && sync && echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null"
  hyperfine_args=(--warmup 0 --min-runs "${min_runs:-5}")
  cache_label="cold"
else
  hyperfine_args=(--warmup 3 --min-runs "${min_runs:-10}")
  cache_label="warm"
fi
hyperfine_args+=(
  --prepare "$prepare_cmd"
  --export-json "$results_dir/hyperfine.json"
  --export-markdown "$results_dir/hyperfine.md"
)

commands=()
for name in "${names[@]}"; do
  hyperfine_args+=(--command-name "$name")
  commands+=("${binaries[$name]} ${arguments[$name]} \"$corpus\"")
done

echo "==> timing ($cache_label cache) over $corpus"
hyperfine "${hyperfine_args[@]}" "${commands[@]}"

rm -f Results.txt

# README-ready table: hyperfine's own markdown export has no version column.
version_args=()
for name in "${names[@]}"; do
  version_args+=("$name" "${versions[$name]}")
done

python3 - "$results_dir/hyperfine.json" "$cache_label" "$results_dir/README.md" "${version_args[@]}" <<'PY'
import json
import sys

results_path, cache_label, out_path = sys.argv[1:4]
versions = dict(zip(sys.argv[4::2], sys.argv[5::2]))

with open(results_path) as handle:
    results = json.load(handle)["results"]

fastest = min(r["mean"] for r in results)
lines = [
    f"| Program ({cache_label} filesystem cache) | Version | Mean [s] | Min [s] | Max [s] |",
    "| :--- | ---: | ---: | ---: | ---: |",
]
for result in results:
    name = result["command"]
    # stddev is null when hyperfine only got a single run out of a command.
    stddev = result.get("stddev")
    mean = f"{result['mean']:.3f}" + (f" ± {stddev:.3f}" if stddev is not None else "")
    if result["mean"] == fastest:
        mean = f"**{mean}**"
    lines.append(
        f"| `{name}` | {versions.get(name, '?')} | {mean} "
        f"| {min(result['times']):.3f} | {max(result['times']):.3f} |"
    )

table = "\n".join(lines)
with open(out_path, "w") as handle:
    handle.write(table + "\n")
print()
print(table)
PY

cp "$corpus/manifest.json" "$results_dir/"
echo
echo "==> results written to $results_dir"

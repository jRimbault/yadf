#!/usr/bin/env bash
#/ Usage: bench-versions.sh [--cold] [--corpus DIR] [--strace] REV [REV...]
#/
#/ Build yadf at each REV (git worktree + cargo build --release), verify they
#/ all produce the same duplicate groups over --corpus, then compare their
#/ runtime with hyperfine. Intended for A/B-ing yadf against itself across
#/ commits, not against other dupe finders (see scripts/bench.sh for that).
#/
#/ Options:
#/   --cold          drop the page cache before each timed run (needs
#/                   passwordless sudo for `tee /proc/sys/vm/drop_caches`);
#/                   default is a warm-cache benchmark
#/   --corpus DIR    directory to scan; generated at .bench-cache/corpus with
#/                   scripts/gen-corpus.py on first use if not given
#/   --strace        also report syscall counts per binary (openat/read/statx/
#/                   newfstatat/getdents64); deterministic, cache-independent
#/   -h, --help      show this help
#/
#/ REV is any git revision, or the literal "dirty" for the working tree as-is.
#/
#/ Note: .cargo/config.toml sets `-C target-cpu=native`, so results are only
#/ comparable across runs made on the SAME machine.
#/
#/ Examples:
#/   bench-versions.sh main dirty
#/   bench-versions.sh --cold --corpus /data/corpus main HEAD
#/   bench-versions.sh --strace main HEAD

set -euo pipefail

usage() {
  grep "^#/" "$0" | cut -c4-
  exit "${1:-0}"
}

repo_root=$(git rev-parse --show-toplevel)
cache_dir="$repo_root/.bench-cache"
bin_dir="$cache_dir/bin"
worktree_dir="$cache_dir/worktrees"
timestamp=$(date +%Y%m%dT%H%M%S)
results_dir="$repo_root/bench-results/$timestamp"

cold=0
corpus=""
do_strace=0
revs=()

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
    --strace)
      do_strace=1
      shift
      ;;
    -h | --help)
      usage 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      echo "unknown option: $1" >&2
      usage 1
      ;;
    *)
      revs+=("$1")
      shift
      ;;
  esac
done
revs+=("$@")

if [[ ${#revs[@]} -lt 1 ]]; then
  echo "error: need at least one REV" >&2
  usage 1
fi

for tool in hyperfine cargo git python3; do
  command -v "$tool" >/dev/null || {
    echo "error: $tool is required but not found in PATH" >&2
    exit 1
  }
done
if [[ "$do_strace" -eq 1 ]] && ! command -v strace >/dev/null; then
  echo "error: --strace requires the strace binary" >&2
  exit 1
fi

if [[ -z "$corpus" ]]; then
  corpus="$cache_dir/corpus"
  if [[ ! -f "$corpus/manifest.json" ]]; then
    echo "==> generating default corpus at $corpus"
    python3 "$repo_root/scripts/gen-corpus.py" \
      --out "$corpus" --seed 42 --files 20000 --dup-ratio 0.1 --collide-prefix 0.05
  fi
fi
if [[ ! -f "$corpus/manifest.json" ]]; then
  echo "error: corpus manifest not found at $corpus/manifest.json" >&2
  exit 1
fi

if [[ "$cold" -eq 1 ]] && ! sudo -n true 2>/dev/null; then
  echo "error: --cold requires passwordless sudo to drop the page cache" >&2
  exit 1
fi

mkdir -p "$bin_dir" "$worktree_dir" "$results_dir"
git -C "$repo_root" worktree prune

# Resolves REV to a built binary path, building and caching it if needed.
# Echoes the binary path on stdout; all progress goes to stderr.
build_rev() {
  local rev="$1" sha short build_dir bin_path
  if [[ "$rev" == "dirty" ]]; then
    echo "==> building working tree (dirty)" >&2
    (cd "$repo_root" && cargo build --release --quiet) >&2
    echo "$repo_root/target/release/yadf"
    return
  fi
  sha=$(git -C "$repo_root" rev-parse "$rev")
  short="${sha:0:12}"
  bin_path="$bin_dir/yadf-$short"
  if [[ -x "$bin_path" ]]; then
    echo "$bin_path"
    return
  fi
  build_dir="$worktree_dir/$short"
  if [[ ! -d "$build_dir" ]]; then
    git -C "$repo_root" worktree add --detach --quiet "$build_dir" "$sha" >&2
  fi
  echo "==> building $rev ($short)" >&2
  (cd "$build_dir" && cargo build --release --quiet) >&2
  cp "$build_dir/target/release/yadf" "$bin_path"
  echo "$bin_path"
}

labels=()
bins=()
for rev in "${revs[@]}"; do
  bin=$(build_rev "$rev")
  labels+=("$rev")
  bins+=("$bin")
done

# Normalizes yadf ldjson output (one duplicate group per line) into a
# sorted, comparable text form: paths sorted within a group, groups sorted
# against each other. Absolute-vs-relative path differences aside, two
# binaries scanning the same corpus must normalize identically.
normalize_output() {
  python3 -c '
import json, sys
groups = [sorted(json.loads(line)) for line in sys.stdin if line.strip()]
for group in sorted(groups):
    print(json.dumps(group))
'
}

echo "==> correctness gate: comparing duplicate groups across all revisions"
baseline_out="$results_dir/baseline.normalized"
for i in "${!bins[@]}"; do
  label="${labels[$i]}"
  bin="${bins[$i]}"
  out_file="$results_dir/${label//\//_}.normalized"
  "$bin" --format ld-json "$corpus" | normalize_output >"$out_file"
  if [[ "$i" -eq 0 ]]; then
    cp "$out_file" "$baseline_out"
  elif ! diff -q "$baseline_out" "$out_file" >/dev/null; then
    echo "error: output of '$label' diverges from '${labels[0]}'" >&2
    diff "$baseline_out" "$out_file" | head -20 >&2
    exit 1
  fi
done

expected_groups=$(python3 -c "import json; print(json.load(open('$corpus/manifest.json'))['duplicate_group_count'])")
actual_groups=$(wc -l <"$baseline_out")
if [[ "$actual_groups" -ne "$expected_groups" ]]; then
  echo "error: found $actual_groups duplicate groups, corpus manifest expects $expected_groups" >&2
  exit 1
fi
echo "    ok: all revisions agree, $actual_groups duplicate groups match manifest"

if [[ "$do_strace" -eq 1 ]]; then
  echo "==> syscall counts"
  for i in "${!bins[@]}"; do
    label="${labels[$i]}"
    bin="${bins[$i]}"
    strace_file="$results_dir/${label//\//_}.strace"
    strace -f -c -w -o "$strace_file" "$bin" "$corpus" >/dev/null
    echo "--- $label ---"
    grep -E '(^ *% time|\bopenat\b|\bread\b|\bclose\b|\bstatx\b|\bnewfstatat\b|\bgetdents64\b|\btotal\b)' "$strace_file" || true
  done
fi

if [[ "$cold" -eq 1 ]]; then
  echo "==> timing (cold cache)"
else
  echo "==> timing (warm cache)"
fi
commands=()
for bin in "${bins[@]}"; do
  commands+=("$bin \"$corpus\"")
done
hyperfine_args=(--export-json "$results_dir/hyperfine.json" --export-markdown "$results_dir/hyperfine.md")
if [[ "$cold" -eq 1 ]]; then
  hyperfine_args+=(--warmup 0 --min-runs 5 --prepare 'sync && echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null')
else
  hyperfine_args+=(--warmup 3 --min-runs 10)
fi
for i in "${!labels[@]}"; do
  hyperfine_args+=(--command-name "${labels[$i]}")
done

hyperfine "${hyperfine_args[@]}" "${commands[@]}"

cp "$corpus/manifest.json" "$results_dir/"
echo "==> results written to $results_dir"
cat "$results_dir/hyperfine.md"

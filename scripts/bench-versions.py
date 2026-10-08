#!/usr/bin/env -S uv run --script --quiet
"""Compare yadf against itself across git revisions.

Build yadf at each REV (git worktree + cargo build --release), verify they all
produce the same duplicate groups over --corpus, then compare their runtime and
peak RSS with hyperfine (>= 2.0). Intended for A/B-ing yadf against itself
across commits, not against other dupe finders (see scripts/bench.py for that).

REV is any git revision, or the literal "dirty" for the working tree as-is.

Note: .cargo/config.toml sets `-C target-cpu=native`, so results are only
comparable across runs made on the SAME machine.

examples:
  bench-versions.py main dirty
  bench-versions.py --cold --corpus /data/corpus main HEAD
  bench-versions.py --strace main HEAD
"""

import argparse
import difflib
import json
import re
import shutil
import subprocess
from pathlib import Path

from pydantic import TypeAdapter

from benchlib import (
    CACHE_DIR,
    DROP_CACHES,
    REPO_ROOT,
    Fatal,
    Model,
    ensure_corpus,
    entrypoint,
    hyperfine,
    load_manifest,
    new_results_dir,
    output,
    peak_rss_mib,
    plus_minus,
    report,
    require_hyperfine_2,
    require_passwordless_sudo,
    require_tools,
    run,
    stdout,
    step,
)

BIN_DIR = CACHE_DIR / "bin"
WORKTREE_DIR = CACHE_DIR / "worktrees"

# Syscalls worth reporting from `strace -c`, plus its header and total lines.
STRACE_LINES = re.compile(
    r"^ *% time|\b(openat|read|close|statx|newfstatat|getdents64|total)\b"
)

GROUP = TypeAdapter(list[str])


class Args(Model):
    cold: bool
    corpus: Path | None
    strace: bool
    revs: tuple[str, ...]


def parse_args() -> Args:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--cold",
        action="store_true",
        help="drop the page cache before each timed run (needs passwordless sudo for "
        "`tee /proc/sys/vm/drop_caches`); default is a warm-cache benchmark",
    )
    parser.add_argument(
        "--corpus",
        type=Path,
        metavar="DIR",
        help="directory to scan; generated at .bench-cache/corpus with "
        "scripts/gen-corpus.py on first use if not given",
    )
    parser.add_argument(
        "--strace",
        action="store_true",
        help="also report syscall counts per binary (openat/read/statx/newfstatat/"
        "getdents64); deterministic, cache-independent",
    )
    parser.add_argument("revs", nargs="+", metavar="REV")
    return Args.model_validate(vars(parser.parse_args()))


def build_rev(rev: str) -> Path:
    """Resolves REV to a built binary, building and caching it if needed."""
    if rev == "dirty":
        step("building working tree (dirty)")
        run("cargo", "build", "--release", "--quiet", cwd=REPO_ROOT)
        return REPO_ROOT / "target" / "release" / "yadf"
    try:
        sha = output(
            "git", "-C", REPO_ROOT, "rev-parse", "--verify", f"{rev}^{{commit}}"
        )
    except subprocess.CalledProcessError as error:
        raise Fatal(f"unknown revision: {rev}") from error
    short = sha.strip()[:12]
    bin_path = BIN_DIR / f"yadf-{short}"
    if bin_path.is_file():
        return bin_path
    build_dir = WORKTREE_DIR / short
    if not build_dir.is_dir():
        run(
            "git",
            "-C",
            REPO_ROOT,
            "worktree",
            "add",
            "--detach",
            "--quiet",
            build_dir,
            short,
        )
    step(f"building {rev} ({short})")
    run("cargo", "build", "--release", "--quiet", cwd=build_dir)
    shutil.copy2(build_dir / "target" / "release" / "yadf", bin_path)
    return bin_path


def normalize(ldjson: str) -> list[str]:
    """Normalizes yadf ldjson output (one duplicate group per line).

    Paths are sorted within a group, and groups against each other: two binaries
    scanning the same corpus must normalize identically.
    """
    groups = sorted(
        sorted(GROUP.validate_json(line)) for line in ldjson.splitlines() if line
    )
    return [json.dumps(group) for group in groups]


def check_outputs(
    labels: list[str], bins: list[Path], corpus: Path, results_dir: Path
) -> None:
    step("correctness gate: comparing duplicate groups across all revisions")
    baseline: list[str] = []
    for i, (label, binary) in enumerate(zip(labels, bins, strict=True)):
        groups = normalize(output(binary, "--format", "ld-json", corpus))
        file_name = f"{label.replace('/', '_')}.normalized"
        (results_dir / file_name).write_text("".join(f"{group}\n" for group in groups))
        if i == 0:
            baseline = groups
            (results_dir / "baseline.normalized").write_text(
                "".join(f"{group}\n" for group in groups)
            )
        elif groups != baseline:
            diff = difflib.unified_diff(baseline, groups, labels[0], label, lineterm="")
            raise Fatal(
                f"output of '{label}' diverges from '{labels[0]}'\n"
                + "\n".join(list(diff)[:20])
            )

    expected = load_manifest(corpus).duplicate_group_count
    if len(baseline) != expected:
        raise Fatal(
            f"found {len(baseline)} duplicate groups, corpus manifest expects {expected}"
        )
    stdout.print(
        f"    ok: all revisions agree, {len(baseline)} duplicate groups match manifest"
    )


def report_syscalls(
    labels: list[str], bins: list[Path], corpus: Path, results_dir: Path
) -> None:
    step("syscall counts")
    for label, binary in zip(labels, bins, strict=True):
        strace_file = results_dir / f"{label.replace('/', '_')}.strace"
        subprocess.run(
            [
                "strace",
                "-f",
                "-c",
                "-w",
                "-o",
                str(strace_file),
                str(binary),
                str(corpus),
            ],
            stdout=subprocess.DEVNULL,
            check=True,
        )
        stdout.print(f"--- {label} ---", markup=False)
        for line in strace_file.read_text().splitlines():
            if STRACE_LINES.search(line):
                stdout.print(line, markup=False)


def main() -> None:
    args = parse_args()
    require_tools("hyperfine", "cargo", "git")
    require_hyperfine_2()
    if args.strace:
        require_tools("strace")
    corpus = ensure_corpus(
        args.corpus,
        *(
            "--seed",
            "42",
            "--files",
            "20000",
            "--dup-ratio",
            "0.1",
            "--collide-prefix",
            "0.05",
        ),
    )
    if args.cold:
        require_passwordless_sudo()

    results_dir = new_results_dir()
    BIN_DIR.mkdir(parents=True, exist_ok=True)
    WORKTREE_DIR.mkdir(parents=True, exist_ok=True)
    run("git", "-C", REPO_ROOT, "worktree", "prune")

    labels = list(args.revs)
    bins = [build_rev(rev) for rev in labels]

    check_outputs(labels, bins, corpus, results_dir)
    if args.strace:
        report_syscalls(labels, bins, corpus, results_dir)

    if args.cold:
        step("timing (cold cache)")
        options = ["--warmup", "0", "--min-runs", "5", "--prepare", DROP_CACHES]
    else:
        step("timing (warm cache)")
        options = ["--warmup", "3", "--min-runs", "10"]
    export = hyperfine(
        [(label, [binary, corpus]) for label, binary in zip(labels, bins, strict=True)],
        options,
        results_dir,
    )

    fastest = min(result.summary.time_wall_clock.mean for result in export.results)
    rows: list[list[str]] = []
    for result in export.results:
        wall = result.summary.time_wall_clock
        rows.append(
            [
                f"`{result.name}`",
                plus_minus(wall, 1e3, 1),
                f"{wall.min * 1e3:.1f}",
                f"{wall.max * 1e3:.1f}",
                f"{wall.mean / fastest:.2f}",
                f"{result.summary.time_user.mean:.2f}",
                f"{result.summary.time_system.mean:.2f}",
                peak_rss_mib(result.summary),
            ]
        )
    report(
        [
            ("Command", "left"),
            ("Mean [ms]", "right"),
            ("Min [ms]", "right"),
            ("Max [ms]", "right"),
            ("Relative", "right"),
            ("User [s]", "right"),
            ("System [s]", "right"),
            ("Peak RSS [MiB]", "right"),
        ],
        rows,
        results_dir / "README.md",
    )

    shutil.copy2(corpus / "manifest.json", results_dir)
    step(f"results written to {results_dir}")


if __name__ == "__main__":
    entrypoint(main)

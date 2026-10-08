#!/usr/bin/env -S uv run --script --quiet
# /// script
# requires-python = ">=3.14"
# dependencies = [
#   "pydantic>=2.13",
#   "rich>=15.0",
# ]
# ///
"""Benchmark yadf against the other duplicate finders.

Times every program over a reproducible synthetic corpus with hyperfine
(>= 2.0), and writes a README-ready markdown table (program, version,
mean/min/max, peak RSS). To compare yadf against itself across commits or
releases, use scripts/bench-versions.py instead.

Every competitor is run so that it looks at the same files yadf does:

  - fclones skips empty, hidden, gitignored and symlinked files by default,
    hence --min 0 --hidden --no-ignore
  - jdupes does not recurse by default, hence -r, and skips empty files
    without -z
  - dupe-krill skips files smaller than the block size (-s) and hardlinks
    matches together unless told not to (-d, dry run)
  - fddf ignores zero length files without -m 0
  - ddh writes a Results.txt in the working directory on every run

Note: .cargo/config.toml sets `-C target-cpu=native`, so results are only
comparable across runs made on the SAME machine.

examples:
  bench.py
  bench.py --install --cold
  bench.py --corpus /data/corpus --min-runs 20
"""

import argparse
import os
import re
import shutil
import subprocess
import tarfile
from dataclasses import dataclass
from http.client import HTTPResponse
from pathlib import Path
from urllib.error import URLError
from urllib.request import Request, urlopen

from pydantic import ValidationError

from benchlib import (
    CACHE_DIR,
    DROP_CACHES,
    REPO_ROOT,
    Fatal,
    Model,
    ensure_corpus,
    entrypoint,
    hyperfine,
    new_results_dir,
    peak_rss_mib,
    plus_minus,
    report,
    require_hyperfine_2,
    require_passwordless_sudo,
    require_tools,
    run,
    stdout,
    step,
    warn,
)

TOOLS_DIR = CACHE_DIR / "tools"

# Corpus the README numbers are quoted against: deterministic given the seed.
CORPUS_ARGS = (
    *("--seed", "42", "--files", "150000", "--dup-ratio", "0.15"),
    *("--collide-prefix", "0.05", "--size-dist", "realistic"),
)

# jdupes (and the libjodycode it links) are built from source release tarballs
# at these pinned versions, so the README numbers are reproducible. Bump them
# deliberately; see https://codeberg.org/jbruchon/jdupes/releases
JDUPES_VERSION = "1.31.2"
LIBJODYCODE_VERSION = "4.1.1"


@dataclass(frozen=True)
class Program:
    name: str
    args: tuple[str, ...]
    # Installed with `cargo install --locked` if published on crates.io.
    crate: bool = False


PROGRAMS = (
    Program("fclones", ("group", "--min", "0", "--hidden", "--no-ignore"), crate=True),
    Program("jdupes", ("-z", "-r")),
    Program("ddh", ("--directories",), crate=True),
    Program("dupe-krill", ("-s", "-d"), crate=True),
    Program("fddf", ("-m", "0"), crate=True),
    Program("yadf", ()),
)


class Args(Model):
    cold: bool
    corpus: Path | None
    install: bool
    min_runs: int | None


class CrateInfo(Model):
    max_stable_version: str


class CrateResponse(Model):
    crate: CrateInfo


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
        "--install",
        action="store_true",
        help="cargo install --locked the crates.io competitors at their latest published "
        "version before benchmarking (the versions pinned in scripts/mise.toml are the "
        "reproducible alternative); jdupes is always built from the pinned source "
        "release into .bench-cache/tools, since no usable binary is packaged",
    )
    parser.add_argument(
        "--min-runs",
        type=int,
        metavar="N",
        help="minimum hyperfine runs per program (default 10 warm, 5 cold)",
    )
    return Args.model_validate(vars(parser.parse_args()))


def crates_io_latest(crate: str) -> str | None:
    """Latest published version of a crate, per the crates.io API."""
    request = Request(
        f"https://crates.io/api/v1/crates/{crate}", headers={"User-Agent": "yadf-bench"}
    )
    try:
        response: HTTPResponse = urlopen(request, timeout=10)
        with response:
            body = response.read()
        return CrateResponse.model_validate_json(body).crate.max_stable_version
    except (URLError, TimeoutError, ValidationError):
        return None


def extract_tarball(url: str, dest: Path) -> None:
    response: HTTPResponse = urlopen(url, timeout=60)
    with response, tarfile.open(fileobj=response, mode="r|gz") as tar:
        tar.extractall(dest, filter="data")


def build_jdupes(version: str, ljc_version: str) -> Path:
    """Builds jdupes from the upstream source release into the tool cache.

    Distro packages lag several minor versions behind (Ubuntu 24.04 ships
    1.20.2), and upstream's own prebuilt binaries hardcode an ELF interpreter
    path (/lib/ld-linux-x86-64.so.2) that does not exist on a usrmerge distro,
    so neither is usable here. jdupes needs libjodycode, which is not packaged
    either; its Makefile picks up a sibling ../libjodycode checkout and can link
    it statically, which is what this does.
    """
    dest = TOOLS_DIR / f"jdupes-{version}" / "jdupes"
    if dest.is_file():
        return dest
    src_dir = TOOLS_DIR / f"src-jdupes-{version}"
    step(f"building jdupes {version} against libjodycode {ljc_version}")
    shutil.rmtree(src_dir, ignore_errors=True)
    src_dir.mkdir(parents=True)
    codeberg = "https://codeberg.org/jbruchon"
    extract_tarball(f"{codeberg}/libjodycode/archive/v{ljc_version}.tar.gz", src_dir)
    extract_tarball(f"{codeberg}/jdupes/archive/v{version}.tar.gz", src_dir)
    jobs = f"-j{os.process_cpu_count() or 1}"
    run("make", "-C", src_dir / "libjodycode", jobs)
    run("make", "-C", src_dir / "jdupes", jobs, "static_jc")
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src_dir / "jdupes" / "jdupes", dest)
    return dest


def version_of(program: str, binary: Path) -> str:
    """Version string, normalized to a bare number for the README table.

    dupe-krill has no --version; it prints "(vX.Y.Z)" in its help banner.
    """
    flag = "--help" if program == "dupe-krill" else "--version"
    banner = subprocess.run(
        [str(binary), flag],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    ).stdout
    first_line = banner.splitlines()[0] if banner else ""
    match = re.search(r"\d+\.\d+\.\d+", first_line)
    return match[0] if match else "?"


def find_binaries(jdupes: Path, yadf: Path) -> dict[str, Path]:
    binaries: dict[str, Path] = {}
    missing: list[str] = []
    for program in PROGRAMS:
        match program.name:
            case "jdupes":
                binaries[program.name] = jdupes
            case "yadf":
                binaries[program.name] = yadf
            case name:
                found = shutil.which(name)
                if found is None:
                    missing.append(name)
                else:
                    binaries[name] = Path(found)
    if missing:
        raise Fatal(
            f"not found in PATH: {' '.join(missing)}\n"
            "       run with --install to install the competitors"
        )
    return binaries


def main() -> None:
    args = parse_args()
    require_tools("hyperfine", "cargo", "git", "make", "cc")
    require_hyperfine_2()
    if args.cold:
        require_passwordless_sudo()

    results_dir = new_results_dir()
    TOOLS_DIR.mkdir(parents=True, exist_ok=True)

    crates = [program.name for program in PROGRAMS if program.crate]
    if args.install:
        for crate in crates:
            step(f"cargo install --locked {crate}")
            run("cargo", "install", "--locked", "--quiet", crate)

    jdupes = build_jdupes(JDUPES_VERSION, LIBJODYCODE_VERSION)

    # yadf is always benchmarked from the working tree, not from whatever happens
    # to be installed in ~/.cargo/bin.
    step("building yadf from the working tree")
    run("cargo", "build", "--release", "--quiet", cwd=REPO_ROOT)
    yadf = REPO_ROOT / "target" / "release" / "yadf"

    corpus = ensure_corpus(args.corpus, *CORPUS_ARGS)
    binaries = find_binaries(jdupes, yadf)

    step("versions under test")
    versions = {name: version_of(name, binary) for name, binary in binaries.items()}
    for name, version in versions.items():
        stdout.print(f"    {name:<12} {version}", markup=False)

    # Competitors published on crates.io should be benchmarked at their latest
    # release; warn rather than fail so an offline run still produces numbers.
    for crate in crates:
        latest = crates_io_latest(crate)
        if latest is not None and latest != versions[crate]:
            warn(f"{crate} {versions[crate]} is installed, {latest} is published")
            warn("re-run with --install to upgrade")

    # ddh drops a Results.txt in the working directory on every run: run in the
    # results directory, and remove it between runs so no program is timed
    # against a dirtied directory.
    prepare = "rm -f Results.txt"
    if args.cold:
        cache_label = "cold"
        prepare += f" && {DROP_CACHES}"
        options = ["--warmup", "0", "--min-runs", str(args.min_runs or 5)]
    else:
        cache_label = "warm"
        options = ["--warmup", "3", "--min-runs", str(args.min_runs or 10)]
    options += ["--prepare", prepare]

    step(f"timing ({cache_label} cache) over {corpus}")
    export = hyperfine(
        [
            (program.name, [binaries[program.name], *program.args, corpus])
            for program in PROGRAMS
        ],
        options,
        results_dir,
        cwd=results_dir,
    )
    (results_dir / "Results.txt").unlink(missing_ok=True)

    # README-ready table: hyperfine's own markdown export has no version column.
    fastest = min(result.summary.time_wall_clock.mean for result in export.results)
    rows: list[list[str]] = []
    for result in export.results:
        wall = result.summary.time_wall_clock
        mean = plus_minus(wall, 1, 3)
        rows.append(
            [
                f"`{result.name}`",
                versions.get(result.name, "?"),
                f"**{mean}**" if wall.mean == fastest else mean,
                f"{wall.min:.3f}",
                f"{wall.max:.3f}",
                peak_rss_mib(result.summary),
            ]
        )
    report(
        [
            (f"Program ({cache_label} filesystem cache)", "left"),
            ("Version", "right"),
            ("Mean [s]", "right"),
            ("Min [s]", "right"),
            ("Max [s]", "right"),
            ("Peak RSS [MiB]", "right"),
        ],
        rows,
        results_dir / "README.md",
    )

    shutil.copy2(corpus / "manifest.json", results_dir)
    step(f"results written to {results_dir}")


if __name__ == "__main__":
    entrypoint(main)

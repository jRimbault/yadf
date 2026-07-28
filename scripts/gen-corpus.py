#!/usr/bin/env python3
"""Deterministic synthetic file-tree generator for benchmarking dupe finders.

Given a seed, always produces a byte-identical tree (same paths, same
content), so runs of scripts/bench-versions.sh are comparable across
machines and across time.

Usage examples:

    gen-corpus.py --out /tmp/corpus --seed 42 --files 200000 \\
        --dup-ratio 0.15 --collide-prefix 0.05

    gen-corpus.py --out /tmp/corpus --seed 1 --files 5000 --size-dist small
"""

from __future__ import annotations

import argparse
import json
import logging
import math
import random
import shutil
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO

logger = logging.getLogger(Path(__file__).stem)

BLOCK_SIZE = 4 * 1024
CHUNK_SIZE = 32 * 1024 * 1024

SIZE_DISTRIBUTIONS: dict[str, tuple[float, float, int]] = {
    # name: (log-normal mu, log-normal sigma, max size in bytes)
    "realistic": (math.log(8 * 1024), 2.5, 500 * 1024 * 1024),
    "small": (math.log(2 * 1024), 1.5, 64 * 1024),
    "large": (math.log(50 * 1024 * 1024), 1.0, 1024 * 1024 * 1024),
}


@dataclass(frozen=True)
class CorpusConfig:
    out: Path
    seed: int
    files: int
    dup_ratio: float
    size_dist: str
    fanout: int
    depth: int
    collide_prefix: float


@dataclass
class PlannedFile:
    directory: int
    name: str
    size: int
    content_key: str  # rng seed material identifying the byte content


@dataclass
class CorpusManifest:
    seed: int
    files: int
    dup_ratio: float
    size_dist: str
    fanout: int
    depth: int
    collide_prefix: float
    duplicate_group_count: int
    duplicate_file_count: int
    collision_pair_count: int
    total_bytes: int
    directories: list[str] = field(default_factory=list)

    def to_json(self) -> str:
        return json.dumps(self.__dict__, indent=2, sort_keys=True)


def main(config: CorpusConfig) -> None:
    rng = random.Random(config.seed)
    if config.out.exists():
        logger.info("removing existing corpus at %s", config.out)
        shutil.rmtree(config.out)
    directories = plan_directories(config, rng)
    for directory in directories:
        directory.mkdir(parents=True, exist_ok=True)

    mu, sigma, size_cap = SIZE_DISTRIBUTIONS[config.size_dist]
    num_dup_files = round(config.files * config.dup_ratio)
    num_collide_files = round(config.files * config.collide_prefix)
    num_unique_files = config.files - num_dup_files - num_collide_files
    if num_unique_files < 0:
        raise ValueError("--dup-ratio and --collide-prefix together exceed 1.0")

    plan, duplicate_groups, collision_pairs = plan_files(
        rng, len(directories), num_unique_files, num_dup_files, num_collide_files, mu, sigma, size_cap
    )
    total_bytes = write_files(directories, plan, config.seed)

    manifest = CorpusManifest(
        seed=config.seed,
        files=len(plan),
        dup_ratio=config.dup_ratio,
        size_dist=config.size_dist,
        fanout=config.fanout,
        depth=config.depth,
        collide_prefix=config.collide_prefix,
        duplicate_group_count=duplicate_groups,
        # Counted off the plan, not off the budget: a leftover file that could
        # not be paired up is emitted as a unique file instead.
        duplicate_file_count=sum(1 for planned in plan if planned.content_key.startswith("dup-")),
        collision_pair_count=collision_pairs,
        total_bytes=total_bytes,
        directories=[str(d.relative_to(config.out)) for d in directories],
    )
    manifest_path = config.out / "manifest.json"
    manifest_path.write_text(manifest.to_json() + "\n")
    logger.info(
        "wrote %d files (%d duplicate groups, %d collision pairs, %.1f MiB) to %s",
        len(plan),
        duplicate_groups,
        collision_pairs,
        total_bytes / (1024 * 1024),
        config.out,
    )


def plan_directories(config: CorpusConfig, rng: random.Random) -> list[Path]:
    """Build a fanout/depth tree of directories under `config.out`."""
    directories = [config.out]
    frontier = [config.out]
    for level in range(config.depth):
        next_frontier = []
        for parent in frontier:
            for i in range(config.fanout):
                child = parent / f"d{level}_{i}"
                directories.append(child)
                next_frontier.append(child)
        frontier = next_frontier
    rng.shuffle(directories)
    return directories


def plan_files(
    rng: random.Random,
    num_directories: int,
    num_unique_files: int,
    num_dup_files: int,
    num_collide_files: int,
    mu: float,
    sigma: float,
    size_cap: int,
) -> tuple[list[PlannedFile], int, int]:
    plan: list[PlannedFile] = []
    file_index = 0

    def next_dir() -> int:
        return rng.randrange(num_directories)

    def next_size(minimum: int = 0) -> int:
        size = int(rng.lognormvariate(mu, sigma))
        return max(minimum, min(size, size_cap))

    # A minimum of MIN_UNIQUE_CONTENT_SIZE bytes keeps every "unique"-labeled
    # file out of the birthday-paradox danger zone: at 1-4 bytes there are
    # only up to 2**32 possible contents, and with tens of thousands of
    # files two of them coincide often enough to silently create a real
    # duplicate the manifest never planned for. 8 bytes (2**64 possibilities)
    # makes that collision probability negligible at any corpus size this
    # generator is meant for.
    min_unique_content_size = 8

    for _ in range(num_unique_files):
        size = next_size(minimum=min_unique_content_size)
        plan.append(PlannedFile(next_dir(), f"u{file_index}", size, f"unique-{file_index}"))
        file_index += 1

    duplicate_groups = 0
    remaining = num_dup_files
    while remaining > 0:
        group_size = min(remaining, rng.randint(2, 5))
        remaining -= group_size
        size = next_size(minimum=min_unique_content_size)
        if group_size < 2:
            # The dup budget can leave a single file over, and one file is a
            # duplicate of nothing: emit it as unique rather than counting a
            # group no dupe finder will ever report.
            plan.append(PlannedFile(next_dir(), f"u{file_index}", size, f"unique-{file_index}"))
            file_index += 1
            break
        content_key = f"dup-{duplicate_groups}"
        for _ in range(group_size):
            plan.append(PlannedFile(next_dir(), f"d{file_index}", size, content_key))
            file_index += 1
        duplicate_groups += 1

    collision_pairs = 0
    remaining = num_collide_files
    while remaining > 0:
        group_size = min(remaining, 2)
        remaining -= group_size
        if group_size < 2:
            size = next_size(minimum=min_unique_content_size)
            plan.append(PlannedFile(next_dir(), f"u{file_index}", size, f"unique-{file_index}"))
            file_index += 1
            break
        size = next_size(minimum=BLOCK_SIZE * 2)
        collision_key = f"collide-{collision_pairs}"
        for member in range(group_size):
            plan.append(
                PlannedFile(next_dir(), f"c{file_index}", size, f"{collision_key}-member-{member}")
            )
            file_index += 1
        collision_pairs += 1

    return plan, duplicate_groups, collision_pairs


def write_files(directories: list[Path], plan: list[PlannedFile], seed: int) -> int:
    total_bytes = 0
    for planned in plan:
        directory = directories[planned.directory]
        path = directory / planned.name
        total_bytes += write_content(path, planned, seed)
    return total_bytes


def write_content(path: Path, planned: PlannedFile, seed: int) -> int:
    """Stream a planned file's bytes to disk, returning how many were written.

    Content is a pure function of (seed, content key), so two files sharing a
    key are byte-identical without either of them being held in memory: a
    27 GB corpus would otherwise need gigabytes of cached duplicate content.
    """
    if planned.content_key.startswith("collide-"):
        # Files in a collision group share their first block and diverge after
        # it, so the prefix is drawn from the group key and the rest from the
        # per-file key.
        group_key, _, _ = planned.content_key.rpartition("-member-")
        with path.open("wb") as handle:
            handle.write(random.Random(f"{seed}:{group_key}").randbytes(BLOCK_SIZE))
            written = BLOCK_SIZE + write_random(
                handle, planned.size - BLOCK_SIZE, random.Random(f"{seed}:{planned.content_key}")
            )
        return written
    with path.open("wb") as handle:
        return write_random(handle, planned.size, random.Random(f"{seed}:{planned.content_key}"))


def write_random(handle: BinaryIO, size: int, rng: random.Random) -> int:
    """Write `size` random bytes in chunks.

    random.randbytes() builds the whole result as one big integer, which both
    peaks at several times the requested size in memory and overflows outright
    past 256 MiB (getrandbits takes a C int bit count), so large files have to
    be written incrementally.
    """
    written = 0
    while written < size:
        chunk = min(CHUNK_SIZE, size - written)
        handle.write(rng.randbytes(chunk))
        written += chunk
    return written


def positive_float_at_most_one(value: str) -> float:
    parsed = float(value)
    if not 0.0 <= parsed <= 1.0:
        raise argparse.ArgumentTypeError(f"{value!r} must be between 0.0 and 1.0")
    return parsed


def parse_args(argv: list[str]) -> tuple[CorpusConfig, bool]:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", type=Path, required=True, help="output directory for the generated corpus")
    parser.add_argument("--seed", type=int, default=0, help="RNG seed, reused for reproducibility")
    parser.add_argument("--files", type=int, default=10_000, help="total number of files to generate")
    parser.add_argument(
        "--dup-ratio",
        type=positive_float_at_most_one,
        default=0.1,
        help="fraction of files that are byte-identical copies of another file",
    )
    parser.add_argument(
        "--size-dist",
        choices=sorted(SIZE_DISTRIBUTIONS),
        default="realistic",
        help="file size distribution",
    )
    parser.add_argument("--fanout", type=int, default=8, help="subdirectories per directory level")
    parser.add_argument("--depth", type=int, default=3, help="directory tree depth")
    parser.add_argument(
        "--collide-prefix",
        type=positive_float_at_most_one,
        default=0.0,
        help="fraction of files that share their first 4 KiB with another file but differ later",
    )
    parser.add_argument("-v", "--verbose", action="store_true", help="enable debug logging")
    args = parser.parse_args(argv)
    return CorpusConfig(
        out=args.out,
        seed=args.seed,
        files=args.files,
        dup_ratio=args.dup_ratio,
        size_dist=args.size_dist,
        fanout=args.fanout,
        depth=args.depth,
        collide_prefix=args.collide_prefix,
    ), args.verbose


if __name__ == "__main__":
    config, verbose = parse_args(sys.argv[1:])
    logging.basicConfig(level=logging.DEBUG if verbose else logging.INFO, format="%(name)s: %(message)s")
    try:
        main(config)
    except KeyboardInterrupt:
        print()

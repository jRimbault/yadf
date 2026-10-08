#!/usr/bin/env python3

import argparse
import functools
import hashlib
import locale
import math
import multiprocessing
import os
import re
import sys
from collections import defaultdict
from collections.abc import Callable, Hashable, Iterable, ValuesView
from enum import StrEnum
from json import dump as jsondump
from typing import Literal, Protocol

from pydantic import BaseModel, ConfigDict

__all__ = ["ScanConfig", "find_dupes", "main", "parse_args"]


type DuplicateGroups = list[list[str]]


class Hash(Protocol):
    def update(self, data: bytes, /) -> None: ...

    def digest(self) -> bytes: ...


class Algorithm(StrEnum):
    BLAKE2B = hashlib.blake2b.__name__
    SHA384 = hashlib.sha384.__name__
    MD5 = hashlib.md5.__name__


class ScanConfig(BaseModel):
    model_config = ConfigDict(frozen=True)

    directories: tuple[str, ...]
    algorithm: Algorithm
    min: int
    max: float
    format: Literal["fdupes", "json", "ldjson"]
    report: bool


def main(args: ScanConfig) -> None:
    full_counter = find_dupes(args)
    partitioned = partition(full_counter, lambda b: len(b) > 1)
    duplicates, uniques = partitioned[True], partitioned[False]
    DISPLAY[args.format](duplicates)
    if args.report:
        duplicates_files = sum(map(len, duplicates))
        files_scanned = len(uniques) + duplicates_files
        print(f"{files_scanned:n} scanned files", file=sys.stderr)
        print(f"{len(uniques):n} unique files", file=sys.stderr)
        print(
            f"{len(duplicates):n} groups of duplicate files ({duplicates_files:n} files)",
            file=sys.stderr,
        )


def find_dupes(config: ScanConfig) -> ValuesView[list[str]]:
    def build_bag(
        key_value_iterable: Iterable[tuple[bytes, str]],
    ) -> dict[bytes, list[str]]:
        bag: defaultdict[bytes, list[str]] = defaultdict(list)
        for key, value in key_value_iterable:
            bag[key].append(value)
        return bag

    walker = (
        file
        for file in (
            os.path.join(path, file)
            for directory in set(config.directories)
            for (path, _, files) in os.walk(directory)
            for file in files
        )
        if config.min <= os.stat(file).st_size <= config.max
    )

    hasher = functools.partial(hash_file, algorithm=HASHERS[config.algorithm])
    with multiprocessing.Pool() as pool:
        tuples = pool.imap_unordered(hasher, walker, chunksize=32)
        return build_bag(tuples).values()


def hash_file(path: str, algorithm: Callable[[], Hash]) -> tuple[bytes, str]:
    hasher = algorithm()
    with open(path, "rb") as fd:
        while True:
            buf = fd.read(4096)
            if len(buf) == 0:
                break
            hasher.update(buf)
    return hasher.digest(), path


def fdupes(duplicates: DuplicateGroups) -> None:
    last = len(duplicates) - 1
    for i, bucket in enumerate(duplicates):
        print(*bucket, sep="\n")
        if i != last:
            print()


def json(duplicates: DuplicateGroups) -> None:
    jsondump(duplicates, fp=sys.stdout)


def ldjson(duplicates: DuplicateGroups) -> None:
    for bucket in duplicates:
        jsondump(bucket, fp=sys.stdout)


DISPLAY: dict[str, Callable[[DuplicateGroups], None]] = {
    fdupes.__name__: fdupes,
    json.__name__: json,
    ldjson.__name__: ldjson,
}

HASHERS: dict[Algorithm, Callable[[], Hash]] = {
    Algorithm.BLAKE2B: hashlib.blake2b,
    Algorithm.SHA384: hashlib.sha384,
    Algorithm.MD5: hashlib.md5,
}


def partition[T, K: Hashable](
    iterable: Iterable[T], predicate: Callable[[T], K]
) -> defaultdict[K, list[T]]:
    results: defaultdict[K, list[T]] = defaultdict(list)
    for item in iterable:
        results[predicate(item)].append(item)
    return results


def parse_args(argv: list[str]) -> ScanConfig:
    units = {"B": 1, "KB": 2**10, "MB": 2**20, "GB": 2**30, "TB": 2**40}

    def byte_size(size: str) -> int:
        size = size.upper()
        if " " not in size:
            size = re.sub(r"([KMGT]?B?)", r" \1", size)
        parts = size.split()
        if len(parts) < 2:
            parts.append("B")
        elif len(parts[1]) < 2:
            parts[1] += "B"
        number, unit = [string.strip() for string in parts]
        return int(float(number) * units[unit])

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "directories",
        help="directories to search",
        default=[os.getcwd()],
        nargs="*",
    )
    parser.add_argument(
        "-r",
        "--report",
        action="store_true",
        help="print human readable report to stderr",
    )
    parser.add_argument(
        "-f",
        "--format",
        choices=DISPLAY.keys(),
        default=next(iter(DISPLAY)),
        help="output format",
    )
    parser.add_argument(
        "-a",
        "--algorithm",
        choices=HASHERS.keys(),
        default=next(iter(HASHERS)),
        help="hashing algorithm",
    )
    parser.add_argument("--min", type=byte_size, default=0)
    parser.add_argument("--max", type=byte_size, default=math.inf)
    return ScanConfig.model_validate(vars(parser.parse_args(argv)))


if __name__ == "__main__":
    locale.setlocale(locale.LC_ALL, "")
    try:
        main(parse_args(sys.argv[1:]))
    except KeyboardInterrupt:
        print()

#!/usr/bin/env python3

import argparse
import hashlib
import locale
import math
import multiprocessing
import os
import pathlib
import pprint
import re
import sys
from collections import defaultdict
from json import dump as jsondump


locale.setlocale(locale.LC_ALL, "")


def main(args):
    full_counter = find_dupes(
        args.directories, HASHERS[args.algorithm], args.min, args.max
    )
    duplicates, uniques = partition(full_counter, lambda b: len(b) > 1)
    DISPLAY[args.format](duplicates)
    if args.report:
        files_scanned = sum(map(len, full_counter))
        duplicates_files = sum(map(len, duplicates))
        print(f"{files_scanned:n} scanned files", file=sys.stderr)
        print(f"{len(uniques):n} unique files", file=sys.stderr)
        print(
            f"{len(duplicates):n} groups of duplicate files ({duplicates_files:n} files)",
            file=sys.stderr,
        )


def find_dupes(directories, algorithm, min=0, max=math.inf):
    def build_bag(key_value_iterable):
        bag = defaultdict(list)
        for key, value in key_value_iterable:
            bag[key].append(value)
        return bag

    walker = (
        file
        for file in (
            pathlib.Path(os.path.join(path, file))
            for directory in set(directories)
            for (path, _, files) in os.walk(directory)
            for file in files
        )
        if min <= file.stat().st_size <= max
    )

    hasher = FileHasher(algorithm)
    with multiprocessing.Pool() as pool:
        tuples = pool.imap_unordered(hasher, walker, chunksize=32)
        return build_bag(tuples).values()


class FileHasher:
    def __init__(self, hasher):
        self.hasher = hasher

    def __call__(self, path):
        hasher = self.hasher()
        with open(path, "rb") as fd:
            while True:
                buf = fd.read(4096)
                if len(buf) == 0:
                    break
                hasher.update(buf)
        return hasher.digest(), path


def fdupes(duplicates):
    last = len(duplicates) - 1
    for (i, bucket) in enumerate(duplicates):
        print(*bucket, sep="\n")
        if i != last:
            print()


def json(duplicates):
    jsondump(duplicates, fp=sys.stdout)


DISPLAY = {
    fdupes.__name__: fdupes,
    json.__name__: json,
}

HASHERS = {
    hashlib.blake2b.__name__: hashlib.blake2b,
    hashlib.sha384.__name__: hashlib.sha384,
    hashlib.md5.__name__: hashlib.md5,
}


def partition(iterable, predicate, map_ok=None, map_nok=None):
    def identity(item):
        return item

    map_ok = map_ok if map_ok is not None else identity
    map_nok = map_nok if map_nok is not None else identity
    trues, falses = [], []
    for item in iterable:
        if predicate(item):
            trues.append(map_ok(item))
        else:
            falses.append(map_nok(item))
    return trues, falses


def parse_args(argv):
    units = {"B": 1, "KB": 2 ** 10, "MB": 2 ** 20, "GB": 2 ** 30, "TB": 2 ** 40}

    def parse_size(size):
        size = size.upper()
        if " " not in size:
            size = re.sub(r"([KMGT]?B?)", r" \1", size)
        number, unit = [string.strip() for string in size.split()]
        if len(unit) == 1:
            unit += "B"
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
    parser.add_argument("--min", type=parse_size, default=0)
    parser.add_argument("--max", type=parse_size, default=math.inf)
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        main(parse_args(sys.argv[1:]))
    except KeyboardInterrupt:
        print()

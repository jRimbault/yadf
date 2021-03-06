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
from json import dump as jsondump


locale.setlocale(locale.LC_ALL, "")


def main(args):
    full_counter = find_dupes(
        args.directories, HASHERS[args.algorithm], args.min, args.max
    )
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


def find_dupes(directories, algorithm, min=0, max=math.inf):
    def build_bag(key_value_iterable):
        bag = defaultdict(list)
        for key, value in key_value_iterable:
            bag[key].append(value)
        return bag

    walker = (
        file
        for file in (
            os.path.join(path, file)
            for directory in set(directories)
            for (path, _, files) in os.walk(directory)
            for file in files
        )
        if min <= os.stat(file).st_size <= max
    )

    hasher = functools.partial(hash_file, algorithm=algorithm)
    with multiprocessing.Pool() as pool:
        tuples = pool.imap_unordered(hasher, walker, chunksize=32)
        return build_bag(tuples).values()


def hash_file(path, algorithm):
    hasher = algorithm()
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


def ldjson(duplicates):
    for bucket in duplicates:
        jsondump(bucket, fp=sys.stdout)


DISPLAY = {
    fdupes.__name__: fdupes,
    json.__name__: json,
    ldjson.__name__: ldjson,
}

HASHERS = {
    hashlib.blake2b.__name__: hashlib.blake2b,
    hashlib.sha384.__name__: hashlib.sha384,
    hashlib.md5.__name__: hashlib.md5,
}


def partition(iterable, predicate):
    results = defaultdict(list)
    for item in iterable:
        results[predicate(item)].append(item)
    return results


def parse_args(argv):
    units = {"B": 1, "KB": 2 ** 10, "MB": 2 ** 20, "GB": 2 ** 30, "TB": 2 ** 40}

    def byte_size(size):
        size = size.upper()
        if " " not in size:
            size = re.sub(r"([KMGT]?B?)", r" \1", size)
        size = size.split()
        if len(size) < 2:
            size.append("B")
        elif len(size[1]) < 2:
            size[1] += "B"
        number, unit = [string.strip() for string in size]
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
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        main(parse_args(sys.argv[1:]))
    except KeyboardInterrupt:
        print()

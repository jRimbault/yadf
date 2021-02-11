#!/usr/bin/env python3

"""Pipe the output of `yadf --format ldjson` into this script.

Either :

yadf -f ldjson > results.ldjson
python3 keep_oldest.py results.ldjson

Or skipping the intermediate file :

yadf -f ldjson | python3 keep_oldest.py

This script is provided as an example meant to be modified and tinkered with.
"""

import fileinput
import itertools
import json
import multiprocessing
import os
import pathlib


def main():
    cleaner = Cleaner(most_recent_modification_date, yield_all_except_first)
    sequential(fileinput.input(), cleaner)


def sequential(ldjson, cleaner):
    for line in ldjson:
        cleaner(line)


def parallel(ldjson, cleaner):
    with multiprocessing.Pool() as pool:
        pool.imap_unordered(cleaner, ldjson)


class Cleaner:
    def __init__(self, key=None, filter=None):
        self.key = key
        self.filter = filter if filter is not None else lambda f: f

    def __call__(self, line):
        files = json.loads(line)
        files.sort(key=self.key)
        # uncomment to actually delete files
        for filename in self.filter(files):
            # os.remove(filename)
            pass


def most_recent_modification_date(filename):
    return pathlib.Path(filename).stat().st_mtime


def yield_all_except_first(files):
    return itertools.islice(files, 0, None)


if __name__ == "__main__":
    main()

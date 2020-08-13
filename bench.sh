#!/usr/bin/env bash

# ddh produces a Results.txt file after each run
#
# rmlint produces a number of files all named rmlint.{ext}
#
# fdupes and jdupes both don't scan recursively by default
#
# dupe-krill skips file smaller than the block size, hence the -s flag,
# and will hardlinks files together, hence the --dry-run flag

hyperfine --warmup 1 \
  --min-runs 10 \
  --export-csv bench.csv \
  --prepare "rm Results.txt rmlint.* || true" \
  "fdupes -r ~" \
  "jdupes -r ~" \
  "rmlint --hidden ~" \
  "ddh ~" \
  "dupe-krill -s -d ~" \
  "fddf ~" \
  "yadf ~"

{
  rm Results.txt rmlint.* || true
} 2> /dev/null

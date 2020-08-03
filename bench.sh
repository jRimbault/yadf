#!/usr/bin/env bash

hyperfine --warmup 1 \
  --min-runs 10 \
  --export-csv bench.csv \
  --prepare "rm Results.txt rmlint.* || true" \
  "yadf ~" \
  "fdupes -r ~" \
  "jdupes -r ~" \
  "ddh ~" \
  "rmlint --hidden ~" \
  "fddf ~" \

{
  rm Results.txt rmlint.* || true
} 2> /dev/null

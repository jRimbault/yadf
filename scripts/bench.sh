#!/usr/bin/env bash

# ddh produces a Results.txt file after each run
#
# rmlint produces a number of files all named rmlint.{ext}
#
# fclones and jdupes both don't scan recursively by default
#
# dupe-krill skips file smaller than the block size, hence the -s flag,
# and will hardlinks files together, hence the --dry-run flag
#
# fddf ignores zero length files

case "$1" in
  "cold")
    prepare_cmd='rm Results.txt rmlint.* || true && echo "free && sync && echo 3 > /proc/sys/vm/drop_caches && free" | sudo sh'
    warmups=0
    ;;
  *)
    prepare_cmd="rm Results.txt rmlint.* || true"
    warmups=5
    ;;
esac

hyperfine --warmup "$warmups" \
  --min-runs 10 \
  --export-markdown export.md \
  --prepare "$prepare_cmd" \
  "fclones group --min 0 ~" \
  "jdupes -z -r ~" \
  "ddh --directories ~" \
  "dupe-krill -s -d ~" \
  "fddf -m 0 ~" \
  "yadf ~"

{
  rm Results.txt rmlint.* || true
} 2> /dev/null

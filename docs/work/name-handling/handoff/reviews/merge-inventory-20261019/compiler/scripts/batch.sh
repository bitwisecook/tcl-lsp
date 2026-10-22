#!/bin/bash
# usage: batch.sh <bin> <prefix> filters...
S=${SOL49_SCRATCH:-/tmp/sol49}
bin=$1; pre=$2; shift 2
for f in "$@"; do
  tag=$pre-$(echo $f | tr -c 'a-zA-Z0-9_\n' '_')
  $S/runlib.sh $bin $tag "$f"
done

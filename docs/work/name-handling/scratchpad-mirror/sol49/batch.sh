#!/bin/bash
# usage: batch.sh <bin> <prefix> filters...
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol49
bin=$1; pre=$2; shift 2
for f in "$@"; do
  tag=$pre-$(echo $f | tr -c 'a-zA-Z0-9_\n' '_')
  $S/runlib.sh $bin $tag "$f"
done

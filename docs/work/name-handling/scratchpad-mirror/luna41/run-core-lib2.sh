#!/usr/bin/env bash
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/luna41
B=$1; tag=$2; shift 2
for m in "$@"; do
  start=$(date +%s)
  TMO=1500 $S/runbin2.sh $B "corelib-$tag-$m" "$m::" > $S/sum-corelib-$tag-$m.txt 2>&1
  echo "$m $(( $(date +%s)-start ))s $(grep 'test result' $S/sum-corelib-$tag-$m.txt | head -1)"
done

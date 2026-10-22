#!/bin/bash
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad
export THREADS=3 TO=3000
for spec in "sccp:sccp::" "optim:optimiser::" "taintm:taint::" "ssa:ssa::" "interp:interprocedural::"; do
  tag=${spec%%:*}; filter=${spec#*:}
  for w in base after; do $S/sol45/runlib.sh $w $tag "$filter"; done
  $S/sol45/cmp.sh $tag
  du -sh $S/sol45/res
done

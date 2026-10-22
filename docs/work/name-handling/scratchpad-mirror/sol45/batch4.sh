#!/bin/bash
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad
export THREADS=4 TO=3000
for spec in "codegen:codegen::" "reginv:registry_invocation" "interp:interprocedural::" "dtests:analyser::diagnostics::tests" "cmdbind:command_binding::" "lowering:lowering::" "helpers:analyser::diagnostics::helpers" "cu:compilation_unit::" "sccp:sccp::" "ssa:ssa::" "gvn:gvn::" "lowhooks:lowering_hooks::" "fp:analyser::diagnostics::fp::" "optim:optimiser::"; do
  tag=${spec%%:*}; filter=${spec#*:}
  [ -f $S/sol45/res/base-$tag.txt ] || $S/sol45/runlib.sh base $tag "$filter"
  $S/sol45/runlib.sh after $tag "$filter"
  $S/sol45/cmp.sh $tag
done

#!/bin/bash
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad
for spec in "fp:analyser::diagnostics::fp::" "security:analyser::diagnostics::security" "cu:compilation_unit::" "gvn:gvn::" "lowhooks:lowering_hooks::" "irchecks:irules_checks::"; do
  tag=${spec%%:*}; filter=${spec#*:}
  for w in base after; do $S/sol45/runlib.sh $w $tag "$filter"; done
  $S/sol45/cmp.sh $tag
done

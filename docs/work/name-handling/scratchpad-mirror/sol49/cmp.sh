#!/bin/bash
# cmp.sh base.txt after.txt : prints regressions (ok->FAILED) and gains
b=$1; a=$2
join <(sed -E 's/^test (.*) \.\.\. (.*)$/\1 \2/' $b | sort) <(sed -E 's/^test (.*) \.\.\. (.*)$/\1 \2/' $a | sort) > /tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol49/.j
echo "REGRESSIONS:"; awk '$2=="ok" && $3=="FAILED"{print "  "$1}' /tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol49/.j
echo "GAINS: $(awk '$2=="FAILED" && $3=="ok"' /tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol49/.j | wc -l)"

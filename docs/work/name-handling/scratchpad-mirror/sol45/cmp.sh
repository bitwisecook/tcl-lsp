#!/bin/bash
R=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol45/res
tag=$1
st() { grep -E " \.\.\. $2\$" $R/$1-$tag.txt | sed -E 's/^test //; s/ \.\.\. [A-Za-z]+$//' | sort; }
echo "== $tag regressions (ok at base, FAILED after):"
comm -12 <(st base ok) <(st after FAILED)
echo "== $tag fixed: $(comm -12 <(st base FAILED) <(st after ok) | wc -l); base ok=$(st base ok | wc -l) after ok=$(st after ok | wc -l)"

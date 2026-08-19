#!/usr/bin/env bash
D=/tmp/claude-0/-home-user-tcl-lsp/49cadaef-dfc0-5f80-9bf3-267da5a707d9/scratchpad/e3
VER="${1:-8.6}"
for f in a1_remove a2_cross a3_parse2 a4_interact a5_guard a6_teardown order legacy info wtrace; do
    p="$D/$f.tcl"
    [ -f "$p" ] || continue
    echo "### $f ($VER)"
    bash "$D/adv.sh" "$p" "$VER" 2>&1 | sed -n '/diff (ref/,$p'
done

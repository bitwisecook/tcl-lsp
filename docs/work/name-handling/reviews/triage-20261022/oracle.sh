#!/bin/bash
# usage: oracle.sh file.tcl  -> one line per engine
f="$1"; d=$(dirname "$f")
for t in tclsh8.4 tclsh8.5 tclsh8.6 tclsh9.0 tclsh9.1 /home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh; do
  out=$(cd "$d" && timeout 10 $t "$f" 2>&1 | tr '\n' '|' | cut -c1-160)
  printf '  %-9s %s\n' "$(basename $t)" "$out"
done

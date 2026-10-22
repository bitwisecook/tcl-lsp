#!/bin/bash
# usage: runfilters.sh EXE TAG filters...
exe=$1; tag=$2; shift 2
L=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-names
cd /home/user/tcl-lsp/rust/tcl-compiler || exit 1
for f in "$@"; do
  TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh timeout 5000 "$exe" "$f" 2>/dev/null | grep -E '^test .* (ok|FAILED)$|^test result' > "$L/$tag-${f%::}.log"
  echo "$f $(tail -1 "$L/$tag-${f%::}.log")"
done

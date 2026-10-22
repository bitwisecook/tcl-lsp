#!/bin/bash
# usage: runint.sh <label> tests...
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad
label=$1; shift
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh
mkdir -p $S/sol45/res
for t in "$@"; do
  cd /home/user/tcl-lsp/rust
  exe=$(timeout 2400 cargo test -p tcl-compiler --test $t --no-run 2>&1 | grep -o "/home/user/tcl-lsp/target/debug/deps/$t-[0-9a-f]*" | head -1)
  [ -z "$exe" ] && { echo "$t: BUILD FAILED"; continue; }
  cd /home/user/tcl-lsp/rust/tcl-compiler
  timeout 2400 $exe --test-threads 2 > $S/sol45/res/$label-$t.log 2>&1
  grep -E "^test .* \.\.\. (ok|FAILED|ignored)$" $S/sol45/res/$label-$t.log | sort > $S/sol45/res/$label-$t.txt
  echo "$t: $(grep -c ' ok$' $S/sol45/res/$label-$t.txt) ok, $(grep -c 'FAILED$' $S/sol45/res/$label-$t.txt) failed"
  rm -f $exe
done

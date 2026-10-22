#!/bin/bash
# usage: runint.sh <label> tests...
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol47
B=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol45/res
label=$1; shift
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh
for t in "$@"; do
  exe=$($S/build.sh $S/int-build-$t.log -p tcl-compiler --test $t | grep "/$t-" | head -1)
  [ -z "$exe" ] && { echo "$t: BUILD FAILED"; continue; }
  cd /home/user/tcl-lsp/rust/tcl-compiler
  timeout 3000 $exe --test-threads 2 2>&1 | grep -aE "^test |test result|^---- |panicked at" | cut -c1-300 | head -c 20000000 > $S/res/$label-$t.log
  grep -aE "^test .* \.\.\. (ok|FAILED|ignored)$" $S/res/$label-$t.log | sort > $S/res/$label-$t.txt
  echo "$t: $(grep -c ' ok$' $S/res/$label-$t.txt) ok, $(grep -c 'FAILED$' $S/res/$label-$t.txt) failed (base $(grep -c ' ok$' $B/base-$t.txt)/$(wc -l < $B/base-$t.txt))"
  $S/cmp.sh $B/base-$t.txt $S/res/$label-$t.txt
  rm -f $exe
done

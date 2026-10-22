#!/bin/bash
# usage: runlib.sh <which: base|after> <tag> <filter> [extra args]
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad
which=$1; tag=$2; filter=$3; shift 3
if [ "$which" = base ]; then bin=$S/base/tcl_compiler_lib; else bin=$S/sol45/after/tcl_compiler_lib; fi
mkdir -p $S/sol45/res
cd /home/user/tcl-lsp/rust/tcl-compiler
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh
timeout ${TO:-2400} $bin "$filter" --test-threads ${THREADS:-2} --skip memoized_ --skip tcltest_aliases_preserve --skip depth_far_past_cap --skip ordinary_large_body --skip many_small_bodies "$@" 2>&1 | grep -aE "^test |test result|^---- |panicked at|^assertion|^  left|^ right" | cut -c1-300 | head -c 20000000 > $S/sol45/res/$which-$tag.log
grep -E "^test .* \.\.\. (ok|FAILED|ignored)$" $S/sol45/res/$which-$tag.log | sort > $S/sol45/res/$which-$tag.txt
echo "$which $tag: $(grep -c ' ok$' $S/sol45/res/$which-$tag.txt) ok / $(grep -c 'FAILED$' $S/sol45/res/$which-$tag.txt) failed; $(grep -c 'test result' $S/sol45/res/$which-$tag.log) result lines"

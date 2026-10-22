#!/bin/bash
# usage: runlib.sh <bin> <tag> <filter> [extra args]
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol49
bin=$1; tag=$2; filter=$3; shift 3
mkdir -p $S/res
cd /home/user/tcl-lsp/rust/tcl-compiler
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh
timeout ${TO:-2400} $bin "$filter" --test-threads ${THREADS:-3} --skip memoized_ --skip tcltest_aliases_preserve --skip depth_far_past_cap --skip ordinary_large_body --skip many_small_bodies --skip literal_flows_to_regexp_in_a_large "$@" 2>&1 | grep -aE "^test |test result|^---- |panicked at|^assertion|^  left|^ right" | cut -c1-300 | head -c 20000000 > $S/res/$tag.log
grep -aE "^test .* \.\.\. (ok|FAILED|ignored)$" $S/res/$tag.log | sort > $S/res/$tag.txt
echo "$tag: $(grep -c ' ok$' $S/res/$tag.txt) ok / $(grep -c 'FAILED$' $S/res/$tag.txt) failed; $(grep -c 'test result' $S/res/$tag.log) result lines"

#!/bin/bash
# usage: runmods.sh <pkg> <tag> ; builds lib tests, runs each top-level module in its own process
set -u
cd /home/user/tcl-lsp
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/lsp
pkg=$1; tag=$2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh
avail=$(df -BM --output=avail / | tail -1 | tr -dc 0-9)
echo "disk avail ${avail}M"
if [ "$avail" -lt 1990 ]; then echo "DISK TOO LOW"; exit 3; fi
cargo test -p $pkg --lib --no-run --message-format=json 2>$S/build-$tag.err > $S/build-$tag.json
rc=$?
exe=$(grep -o '"executable":"[^"]*"' $S/build-$tag.json | grep -v null | tail -1 | cut -d'"' -f4)
rm -f $S/build-$tag.json
if [ $rc -ne 0 ] || [ -z "$exe" ]; then echo "BUILD FAILED"; exit 2; fi
echo "exe $exe"
cd /home/user/tcl-lsp/rust/$pkg
$exe --list 2>/dev/null | grep ": test$" | sed 's/: test$//' > $S/list-$tag.txt
mkdir -p $S/per-$tag
export EXE=$exe PERDIR=$S/per-$tag
cat $S/list-$tag.txt | xargs -P ${PAR:-3} -I{} bash -c 'out=$PERDIR/$(echo "{}" | tr ":/" "__").log; timeout ${TESTTIMEOUT:-400} $EXE --exact "{}" > $out 2>&1; rc=$?; if grep -q "test result: ok. 1 passed" $out; then echo "ok {}"; elif [ $rc -eq 124 ]; then echo "TIMEOUT {}"; elif [ $rc -eq 134 ] && grep -q " ... ok" $out; then echo "ok-abort {}"; else echo "FAILED {} rc=$rc"; fi' > $S/status-$tag.txt
rm -f "$exe"
echo "TOTAL ok=$(grep -c '^ok ' $S/status-$tag.txt) ok-abort=$(grep -c '^ok-abort ' $S/status-$tag.txt) fail=$(grep -c '^FAILED ' $S/status-$tag.txt) timeout=$(grep -c '^TIMEOUT ' $S/status-$tag.txt) listed=$(wc -l < $S/list-$tag.txt)"

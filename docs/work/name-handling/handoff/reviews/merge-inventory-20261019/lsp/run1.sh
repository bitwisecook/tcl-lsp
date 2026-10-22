#!/bin/bash
# usage: run1.sh <pkg> <target-kind> <name> <tag> [test-filter...]
# target-kind: test | lib
set -u
cd /home/user/tcl-lsp
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/lsp
pkg=$1; kind=$2; name=$3; tag=$4; shift 4
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh
avail=$(df -BM --output=avail / | tail -1 | tr -dc 0-9)
echo "disk avail ${avail}M"
if [ "$avail" -lt 1990 ]; then echo "DISK TOO LOW"; exit 3; fi
if [ "$kind" = lib ]; then sel="--lib"; else sel="--test $name"; fi
cargo test -p $pkg $sel --no-run --message-format=json 2>$S/build-$tag.err > $S/build-$tag.json
rc=$?
exe=$(grep -o '"executable":"[^"]*"' $S/build-$tag.json | grep -v null | tail -1 | cut -d'"' -f4)
rm -f $S/build-$tag.json
if [ $rc -ne 0 ] || [ -z "$exe" ]; then echo "BUILD FAILED rc=$rc"; grep -E "^error" -A5 $S/build-$tag.err | head -40; exit 2; fi
echo "exe $exe $(du -m $exe | cut -f1)M"
cd /home/user/tcl-lsp/rust/$pkg
timeout ${TIMEOUT:-3000} $exe --test-threads=${THREADS:-2} "$@" > $S/out-$tag.log 2>&1
echo "exit $?"
rm -f "$exe"
grep -E "^test .* \.\.\. (ok|FAILED|ignored)" $S/out-$tag.log | sed -E 's/^test (.*) \.\.\. (.*)$/\2 \1/' | sort > $S/status-$tag.txt
grep -E "^test result" $S/out-$tag.log
echo "ok=$(grep -c '^ok ' $S/status-$tag.txt) fail=$(grep -c '^FAILED ' $S/status-$tag.txt)"

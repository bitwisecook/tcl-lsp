#!/bin/bash
# usage: run_target.sh NAME "cargo test args" [test-binary args...]
set -u
NAME=$1; CARGS=$2; shift 2
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/compiler-codegen/res
cd /home/user/tcl-lsp
until [ $(df -B1M / | tail -1 | awk '{print $4}') -ge 2350 ]; do sleep 30; done
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh
timeout 5400 cargo test $CARGS --no-run 2>&1 | grep -E "^(error|warning)|Executable|-->" > $S/$NAME-build.txt
EXE=$(grep -o "(target/[^)]*)" $S/$NAME-build.txt | tail -1 | tr -d '()')
if [ -z "$EXE" ]; then echo "BUILD FAILED $NAME"; cat $S/$NAME-build.txt; exit 1; fi
CRATE_DIR=$(grep -o "Executable [^ ]* [^ ]*" $S/$NAME-build.txt | tail -1)
DIR=$(echo "$CARGS" | grep -o "\-p [^ ]*" | awk '{print $2}')
case $DIR in tcl-*) D=rust/$DIR;; *) D=.;; esac
cd /home/user/tcl-lsp/$D
for mode in base after; do
  if [ $mode = base ]; then export ZZ_CCG_BASELINE=1; else unset ZZ_CCG_BASELINE; fi
  timeout 7000 /home/user/tcl-lsp/$EXE "$@" 2>&1 | grep -aE "^test .*\.\.\. |^test result|panicked at" | head -c 20000000 > $S/$NAME-$mode.txt
done
if [ -n "${KEEP:-}" ]; then echo "$EXE" > $S/$NAME-exe.txt; else rm -f /home/user/tcl-lsp/$EXE; fi
for mode in base after; do echo "$NAME $mode: $(grep -a 'test result' $S/$NAME-$mode.txt | tail -1)"; done
grep -a "^test .* ok$" $S/$NAME-base.txt | sed 's/ \.\.\. ok//' | sort > $S/$NAME-base-ok.txt
grep -a "^test .* ok$" $S/$NAME-after.txt | sed 's/ \.\.\. ok//' | sort > $S/$NAME-after-ok.txt
echo "pass->fail:"; comm -23 $S/$NAME-base-ok.txt $S/$NAME-after-ok.txt
echo "fail->pass count: $(comm -13 $S/$NAME-base-ok.txt $S/$NAME-after-ok.txt | wc -l)"

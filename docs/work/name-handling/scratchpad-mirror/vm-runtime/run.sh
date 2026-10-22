#!/bin/bash
# usage: run.sh TAG PKG TARGETFLAG... ; env: THREADS, TMO, KEEP, FILTER
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/vm-runtime
TAG=$1; PKG=$2; shift 2
NAME=$(echo "$PKG-$*" | tr ' /' '__')
cd /home/user/tcl-lsp
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh
export TCL_LSP_TCLSH90=/root/.local/bin/tclsh9.0 TCL_REQUIRE_SPECTCL_COMPAT=1
FREE=$(df --output=avail -k / | tail -1)
if [ "$FREE" -lt 2000000 ]; then echo "$NAME: DISK LOW ($FREE KB)" | tee -a $S/runs/$TAG.summary; exit 3; fi
OUT=$(cargo test --locked -p $PKG "$@" --no-run -j 2 2>&1)
EXE=$(echo "$OUT" | grep -o '(target/debug/deps/[^)]*)' | tr -d '()' | tail -1)
if [ -z "$EXE" ]; then echo "$NAME: BUILD FAILED" | tee -a $S/runs/$TAG.summary; echo "$OUT" | grep -E "^error|^warning" -A7 | head -80 > $S/runs/$TAG.$NAME.build; exit 1; fi
echo "$OUT" | grep -E "^warning" -A5 | head -40 > $S/runs/$TAG.$NAME.warn
DIR=rust/$PKG; [ -d $DIR ] || DIR=.
START=$(date +%s)
( cd $DIR && timeout ${TMO:-2400} /home/user/tcl-lsp/$EXE $FILTER --test-threads ${THREADS:-2} 2>&1 ) | grep -E "^test |panicked|test result|^(left|right|  left|  right)|assertion|^Error|called .Result" -A2 | cut -c1-800 | head -c 20000000 > $S/runs/$TAG.$NAME.log
END=$(date +%s)
RES=$(grep "test result" $S/runs/$TAG.$NAME.log | tail -1)
grep -E "^test .* (FAILED|ok)$" $S/runs/$TAG.$NAME.log | sort > $S/runs/$TAG.$NAME.status
echo "$NAME: $RES (wall $((END-START))s)" | tee -a $S/runs/$TAG.summary
[ -n "$KEEP" ] || rm -f /home/user/tcl-lsp/$EXE

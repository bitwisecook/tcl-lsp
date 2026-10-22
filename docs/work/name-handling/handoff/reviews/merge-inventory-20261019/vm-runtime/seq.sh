#!/bin/bash
# usage: seq.sh TAG "pkg:test" ...   full logs capped at 3 MB, binaries deleted
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/vm-runtime
TAG=$1; shift
cd /home/user/tcl-lsp
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh TCL_LSP_TCLSH90=/root/.local/bin/tclsh9.0 TCL_REQUIRE_SPECTCL_COMPAT=1
for spec in "$@"; do
  pkg=${spec%%:*}; t=${spec#*:}
  until [ $(df --output=avail -k / | tail -1) -gt 2000000 ]; do sleep 20; done
  OUT=$(timeout 3000 cargo test --locked -p $pkg --test $t --no-run -j 2 2>&1)
  EXE=$(echo "$OUT" | grep -o '(target/debug/deps/[^)]*)' | tr -d '()' | tail -1)
  if [ -z "$EXE" ]; then echo "$pkg:$t BUILD FAILED" >> $S/runs/$TAG.summary; echo "$OUT" | grep -E "^(error|warning)" -A7 | head -60 > $S/runs/$TAG.$t.build; continue; fi
  echo "$OUT" | grep -E "^warning" -A5 | head -30 > $S/runs/$TAG.$t.warn
  ST=$(date +%s)
  (cd rust/$pkg && timeout ${TMO:-3000} /home/user/tcl-lsp/$EXE $FILTER --test-threads ${THREADS:-2} 2>&1 | head -c 3000000 > $S/runs/$TAG.$t.full.log)
  rm -f /home/user/tcl-lsp/$EXE
  echo "$pkg:$t $(grep 'test result' $S/runs/$TAG.$t.full.log | tail -1) (wall $(( $(date +%s)-ST ))s)" >> $S/runs/$TAG.summary
  grep -E "^test .* (ok|FAILED)$" $S/runs/$TAG.$t.full.log | sort > $S/runs/$TAG.$t.status
done
echo DONE >> $S/runs/$TAG.summary

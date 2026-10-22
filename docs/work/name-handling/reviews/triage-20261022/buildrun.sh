#!/bin/bash
# usage: buildrun.sh <log> [test args...]   -- build, run and delete the probe executable in one command
log=$1; shift
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/triage
cd /home/user/tcl-lsp
while true; do
  avail=$(df --output=avail -B1M / | tail -1)
  if [ "$avail" -ge 2400 ]; then break; fi
  echo "$(date +%T) waiting avail=${avail}M" >> $S/$log.wait
  sleep 15
done
echo "start $(date) avail=${avail}M" > $S/$log.log
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh ZZ_SCRATCH=$S/out
timeout 5400 cargo test -p tcl-spectcl --test zz_review_probe -- --nocapture --test-threads=1 "$@" >> $S/$log.log 2>&1
echo "exit=$? $(date)" >> $S/$log.log
rm -f target/debug/deps/zz_review_probe-*[0-9a-f] 2>/dev/null
ls target/debug/deps/ | grep zz_review_probe >> $S/$log.log
echo "cleaned" >> $S/$log.log

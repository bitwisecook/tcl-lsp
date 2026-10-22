#!/bin/bash
# usage: build.sh <crate> <testname> <logname>
crate=$1; t=$2; log=$3
S=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/triage
cd /home/user/tcl-lsp
while true; do
  avail=$(df --output=avail -B1M / | tail -1)
  busy=$(pgrep -f 'rustc --crate-name' | wc -l)
  if [ "$avail" -ge 2150 ]; then break; fi
  echo "$(date +%T) waiting avail=${avail}M rustc=$busy" >> $S/$log.wait
  sleep 15
done
echo "start $(date) avail=${avail}M" > $S/$log.build
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo test -p $crate --test $t --no-run >> $S/$log.build 2>&1
echo "exit=$? $(date)" >> $S/$log.build

#!/bin/bash
# usage: run-reg.sh <outname>
cd /home/user/tcl-lsp
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0
OUT=/tmp/claude-0/-home-user-tcl-lsp/95d74e4c-e1a2-5809-ae6c-93553558eeec/scratchpad/bl/$1
cargo nextest run -p tcl-registry -p tcl-spectcl -p tcl-spec-studio -p tcl-cshim --lib --tests --no-fail-fast --status-level fail --final-status-level none --failure-output never -E 'not test(/codegen_stamps::every_shape/)' > $OUT.log 2>&1
echo "exit $?" >> $OUT.log
grep -E "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV|SIGKILL)" $OUT.log | sed -E 's/^\s+\S+ \[[^]]*\] \([^)]*\) //' | awk '{print $NF}' | sed -E 's/.*:://' | sort -u > $OUT.fail
grep -E "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV|SIGKILL)" $OUT.log | sed -E 's/^\s+\S+ \[[^]]*\] \([^)]*\) //' | sort -u > $OUT.full
wc -l $OUT.fail

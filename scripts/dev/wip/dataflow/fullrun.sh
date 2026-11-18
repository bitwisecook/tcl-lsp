#!/bin/bash
# usage: fullrun.sh <tag>
S=/tmp/claude-0/-home-user-tcl-lsp/8d622143-3532-5414-9b9f-0ba860598aa9/scratchpad
source $S/env.sh
cargo nextest run -p tcl-compiler --no-fail-fast --status-level fail --final-status-level none --failure-output never > $S/run-$1.log 2>&1
grep -E "^\s+FAIL" $S/run-$1.log | awk '{print $NF}' | sort -u > $S/fail-$1.txt
echo "failing $(wc -l < $S/fail-$1.txt)"
echo "new:"; comm -13 $S/fail-base.txt $S/fail-$1.txt
echo "fixed: $(comm -23 $S/fail-base.txt $S/fail-$1.txt | wc -l)"
grep -E "Summary|error\[" $S/run-$1.log | head

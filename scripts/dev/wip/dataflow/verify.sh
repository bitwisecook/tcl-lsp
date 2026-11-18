#!/bin/bash
S=/tmp/claude-0/-home-user-tcl-lsp/8d622143-3532-5414-9b9f-0ba860598aa9/scratchpad
source $S/env.sh
export CARGO_BUILD_JOBS=2
T=$1
{
cargo clippy -p tcl-compiler -p tcl-codegen --all-targets -- -D warnings -A clippy::assert_is_empty 2>&1 | grep -E "^(error|warning)" | sort | uniq -c | head; echo "clippy exit ${PIPESTATUS[0]}"
$S/fullrun.sh $T
echo "new vs p6:"; comm -13 $S/fail-p6.txt $S/fail-$T.txt
echo "fixed vs p6:"; comm -23 $S/fail-p6.txt $S/fail-$T.txt
cargo nextest run -p tcl-codegen --test it_3 -E 'test(/release_optimiser_safety::/)' --no-fail-fast --status-level fail --final-status-level none --failure-output never 2>&1 | grep -E "FAIL \[|Summary|error\["
make xtask-gates > $S/gates-$T.log 2>&1; echo "gates exit $?"; tail -5 $S/gates-$T.log
} > $S/$T.out 2>&1

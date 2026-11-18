#!/bin/bash
cd /home/user/tcl-lsp
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0 CARGO_INCREMENTAL=0
OUT=/tmp/claude-0/-home-user-tcl-lsp/95d74e4c-e1a2-5809-ae6c-93553558eeec/scratchpad/bl/$1
cargo nextest run -p tcl-registry -p tcl-spectcl -p tcl-spec-studio -p tcl-compiler --lib --tests --no-fail-fast --status-level fail --final-status-level none --failure-output never -E '(package(tcl-registry) | package(tcl-spectcl) | package(tcl-spec-studio) | (package(tcl-compiler) & test(/gvn::|side_effects|interprocedural|mixed_region_plan|semantic_analysis|optimiser::elimination|registry_invocation|purity|pure/))) & not test(/codegen_stamps::every_shape/)' > $OUT.log 2>&1
grep -E "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV|SIGKILL)" $OUT.log | sed -E 's/^\s+\S+ \[[^]]*\] \([^)]*\) //' | sort -u > $OUT.full
grep Summary $OUT.log

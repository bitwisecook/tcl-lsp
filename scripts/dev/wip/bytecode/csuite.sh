#!/usr/bin/env bash
cd "$(git rev-parse --show-toplevel)"
export TCL_LSP_AGENT_INCREMENTAL=0
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0 CARGO_BUILD_JOBS=3
S=${OUT:-$PWD/target/bytecode-wip}; mkdir -p "$S"
L=$S/$1.log
cargo nextest run -p tcl-compiler -p tcl-codegen --no-fail-fast --status-level fail --final-status-level fail --failure-output never > $L 2>&1
grep -E '^\s+(FAIL|SIGKILL|SIGSEGV|SIGABRT|TIMEOUT) ' $L | awk '{print $NF}' | sort -u > $S/$1.fail
wc -l < $S/$1.fail; grep Summary $L

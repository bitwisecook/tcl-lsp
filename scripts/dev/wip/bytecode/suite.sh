#!/usr/bin/env bash
# usage: suite.sh <label>
cd "$(git rev-parse --show-toplevel)"
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0
S=${OUT:-$PWD/target/bytecode-wip}; mkdir -p "$S"
L=$S/$1.log
cargo nextest run -p tcl-bytecode -p tcl-codegen -p tcl-vm -p tcl-registry -p tcl-explorer -p tcl-cli --no-fail-fast --status-level fail --final-status-level fail --failure-output never > $L 2>&1
grep -E '^\s+(FAIL|SIGKILL|SIGSEGV|SIGABRT|TIMEOUT) ' $L | awk '{print $NF}' | sort -u > $S/$1.fail
wc -l < $S/$1.fail
tail -3 $L

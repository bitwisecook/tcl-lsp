#!/bin/bash
# Rebuild, run and delete the existing live-fold negative regression.
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
tag="${1:?provide a fresh log tag}"
log="$E/$tag.log"
[ ! -e "$log" ] || exit 2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH="$PWD/tmp/jimtcl-oracle/jimsh"
target_dir="${CARGO_TARGET_DIR:-target}"
cleanup() { rm -f "$target_dir"/debug/deps/variable_reference_boundaries-*; }
trap cleanup EXIT
{
    date -u
    git rev-parse HEAD
    sha256sum rust/tcl-spectcl/tests/variable_reference_boundaries.rs rust/tcl-compiler/src/const_subst.rs
    df -h /
    cargo test --locked -p tcl-spectcl --test variable_reference_boundaries a_live_fold_keeps_declared_effects_and_unclosed_arguments -- --nocapture --test-threads=1
    status=$?
    cleanup
    printf 'exit=%s cleaned=%s\n' "$status" "$(date -u)"
} > "$log" 2>&1
exit "$status"

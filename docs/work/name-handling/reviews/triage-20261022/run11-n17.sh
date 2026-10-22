#!/bin/bash
# Fresh custom contrast plus the product's declared-callback regression.
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
[ ! -e "$E/run11-n17.log" ] || exit 2
[ ! -e "$E/run11-variable-boundaries.log" ] || exit 2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH="$PWD/tmp/jimtcl-oracle/jimsh"
bash "$E/run-focused.sh" run11-n17 n17_live_hook
focused_status=$?
log="$E/run11-variable-boundaries.log"
target_dir="${CARGO_TARGET_DIR:-target}"
cleanup() { rm -f "$target_dir"/debug/deps/variable_reference_boundaries-*; }
trap cleanup EXIT
{ date -u; git rev-parse HEAD; sha256sum rust/tcl-spectcl/tests/variable_reference_boundaries.rs; df -h /; } > "$log"
timeout 1200 cargo test --locked -p tcl-spectcl --test variable_reference_boundaries -- --nocapture --test-threads=1 >> "$log" 2>&1
boundary_status=$?
cleanup
printf 'exit-boundaries=%s cleaned=%s\n' "$boundary_status" "$(date -u)" >> "$log"
if [ "$focused_status" -ne 0 ] || [ "$boundary_status" -ne 0 ]; then exit 1; fi

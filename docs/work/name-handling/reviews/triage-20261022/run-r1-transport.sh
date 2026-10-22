#!/bin/bash
# Run after the active full replay has finished; uses the same authorised probe path.
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
log="$E/${1:-run7-r1-transport}.log"
[ ! -e "$log" ] || exit 2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH="$PWD/tmp/jimtcl-oracle/jimsh"
target_dir="${CARGO_TARGET_DIR:-target}"
cleanup() {
    rm -f rust/tcl-spectcl/tests/zz_review_probe.rs "$target_dir"/debug/deps/zz_review_probe-*
}
trap cleanup EXIT
cp "$E/r1-transport-probe.rs.txt" rust/tcl-spectcl/tests/zz_review_probe.rs
{ date -u; git rev-parse HEAD; sha256sum rust/tcl-spectcl/tests/zz_review_probe.rs; df -h /; } > "$log"
timeout 1200 cargo test --locked -p tcl-spectcl --test zz_review_probe -- --nocapture --test-threads=1 >> "$log" 2>&1
status=$?
printf 'exit-main=%s\n' "$status" >> "$log"
cleanup
exit "$status"

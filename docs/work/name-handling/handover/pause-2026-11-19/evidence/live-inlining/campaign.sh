#!/bin/bash
set -eu
# Supply independent, clean checkouts on work for the baseline and two patches.
: "${LIVE_CLEAN_CHECKOUT:?set the baseline work checkout}"
: "${LIVE_PATCH_CHECKOUT:?set the patched work checkout}"
: "${LIVE_EVIDENCE_DIR:?set an absolute output directory}"
: "${TCL_LSP_REQUIRE_ALL_TCL_ORACLES:?source the verified oracle environment first}"
test "$TCL_LSP_REQUIRE_ALL_TCL_ORACLES" = 1
test "${TCL_LSP_REQUIRE_JIM_ORACLE:-}" = 1
test "${TCL_LSP_REQUIRE_TCLSH:-}" = 1
fixture_generator=$(cd "$(dirname "$0")" && pwd)/campaign.py
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_BUILD_JOBS=1
out=$LIVE_EVIDENCE_DIR
mkdir -p "$out"
run_suite() {
 local root=$1 phase=$2 status=0
 (cd "$root" && cargo nextest run --offline --locked -p tcl-spectcl --test it_5 -E 'test(codegen_stamps::live_differential)' --no-fail-fast --status-level all --final-status-level all --test-threads 1 --failure-output immediate) >"$out/$phase.log" 2>&1 || status=$?
 echo "$status" >"$out/$phase.status"
 rg 'Summary \[' "$out/$phase.log"
}
if [[ "${1:-}" == before ]]; then
  test -z "$(git -C "$LIVE_CLEAN_CHECKOUT" status --porcelain)"
  git -C "$LIVE_CLEAN_CHECKOUT" rev-parse HEAD >"$out/clean-source.txt"
  run_suite "$LIVE_CLEAN_CHECKOUT" clean-off
  exit 0
fi
cd "$LIVE_PATCH_CHECKOUT"
test "$(git rev-parse HEAD~2)" = "$(git -C "$LIVE_CLEAN_CHECKOUT" rev-parse HEAD)"
test -z "$(git status --porcelain)"
service=rust/tcl-codegen/src/compile_service.rs
fixture=rust/tcl-spectcl/tests/it_5/codegen_stamps/live_differential.rs
cleanup() { git restore -- "$service" "$fixture"; }
trap cleanup EXIT
for phase in 'off' 'on'; do
  cleanup
  if [[ "$phase" == on ]]; then sed -i 's/LIVE_REFERENCE_INLINING: bool = false/LIVE_REFERENCE_INLINING: bool = true/' "$service"; fi
  run_suite "$LIVE_PATCH_CHECKOUT" "after-$phase"
  python "$fixture_generator" "$fixture" "$out/generated-cases.json"
  run_suite "$LIVE_PATCH_CHECKOUT" "pack-fuzz-$phase"
done

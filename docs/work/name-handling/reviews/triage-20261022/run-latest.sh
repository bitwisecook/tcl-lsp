#!/bin/bash
# Replay every semantic probe on the latest integrated tree.
# Core tests share a serial process; hook-mutating tests and seams are isolated.
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
tag="${1:?provide a fresh run tag}"
log="$E/$tag.log"
[ ! -e "$log" ] || exit 2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH="$PWD/tmp/jimtcl-oracle/jimsh"
export ZZ_SCRATCH="$PWD/$E/out/$tag"
mkdir -p "$ZZ_SCRATCH"
target_dir="${CARGO_TARGET_DIR:-target}"
cleanup() { rm -f rust/tcl-spectcl/tests/zz_review_probe.rs "$target_dir"/debug/deps/zz_review_probe-* "$target_dir"/debug/deps/pack_source_e2e-*; }
trap cleanup EXIT
cp "$E/final-probe.rs.txt" rust/tcl-spectcl/tests/zz_review_probe.rs
{ date -u; git rev-parse HEAD; sha256sum rust/tcl-spectcl/tests/zz_review_probe.rs; df -h /; } > "$log"
cargo test --locked -p tcl-spectcl --test zz_review_probe --no-run --message-format=json > "$E/$tag-build.json" 2> "$E/$tag-build.log"
status=$?
if [ "$status" -ne 0 ]; then printf 'build-exit=%s\n' "$status" >> "$log"; exit "$status"; fi
exe="$(python3 - "$E/$tag-build.json" <<'PY'
import json,sys
items=[json.loads(line) for line in open(sys.argv[1]) if line.strip()]
matches=[i['executable'] for i in items if i.get('reason')=='compiler-artifact' and i.get('target',{}).get('name')=='zz_review_probe' and i.get('executable')]
assert len(matches)==1,matches
print(matches[0])
PY
)"
[ -x "$exe" ] || exit 2
rm -f rust/tcl-spectcl/tests/zz_review_probe.rs
failed=0
run_group() {
    local label="$1" budget="$2"
    shift 2
    printf '\nGROUP %s budget=%ss %s\n' "$label" "$budget" "$(date -u)" >> "$log"
    timeout "$budget" "$exe" "$@" --nocapture --test-threads=1 >> "$log" 2>&1
    local status=$?
    printf 'GROUP_EXIT %s exit=%s %s\n' "$label" "$status" "$(date -u)" >> "$log"
    if [ "$status" -ne 0 ]; then failed=1; fi
}
run_group rename 1200 rename_with_registry
run_group core 2400 --skip scaling_ --skip rename_with_registry --skip n17_ --skip seam_
run_group scaling 900 scaling_latest_small_rows
run_group hooks 1200 n17_
run_group seams 600 seam_
cleanup
{ date -u; git rev-parse HEAD; sha256sum rust/tcl-compiler/src/const_subst.rs; df -h /; } >> "$log"
cargo test --locked -p tcl-spectcl --test pack_source_e2e a_pack_file_folds_a_call_site_in_the_optimiser -- --nocapture --test-threads=1 >> "$log" 2>&1
status=$?
rm -f "$target_dir"/debug/deps/pack_source_e2e-*
printf 'E2E_EXIT=%s\n' "$status" >> "$log"
if [ "$status" -ne 0 ]; then failed=1; fi
printf 'cleaned %s\n' "$(date -u)" >> "$log"
exit "$failed"

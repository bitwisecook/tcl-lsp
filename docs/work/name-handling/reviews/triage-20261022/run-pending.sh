#!/bin/bash
# Build once, run bounded pending cases, and delete the executable in this command.
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
tag="${1:-run7-pending}"
log="$E/$tag.log"
[ ! -e "$log" ] || exit 2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH="$PWD/tmp/jimtcl-oracle/jimsh"
export ZZ_SCRATCH="$PWD/$E/out/$tag"
mkdir -p "$ZZ_SCRATCH"
target_dir="${CARGO_TARGET_DIR:-target}"
exe=""
cleanup() {
    rm -f rust/tcl-spectcl/tests/zz_review_probe.rs "$target_dir"/debug/deps/zz_review_probe-*
}
trap cleanup EXIT
cp "$E/pending-probe.rs.txt" rust/tcl-spectcl/tests/zz_review_probe.rs
{ date -u; git rev-parse HEAD; sha256sum rust/tcl-spectcl/tests/zz_review_probe.rs; df -h /; } > "$log"
cargo test --locked -p tcl-spectcl --test zz_review_probe --no-run --message-format=json > "$E/$tag-build.json" 2> "$E/$tag-build.log"
build_rc=$?
if [ "$build_rc" -ne 0 ]; then
    printf 'build-exit=%s\n' "$build_rc" >> "$log"
    exit "$build_rc"
fi
exe="$(python3 - "$E/$tag-build.json" <<'PY'
import json, sys
matches=[]
for line in open(sys.argv[1]):
    try: item=json.loads(line)
    except ValueError: continue
    if item.get('reason')=='compiler-artifact' and item.get('target',{}).get('name')=='zz_review_probe' and item.get('executable'):
        matches.append(item['executable'])
assert len(matches)==1, matches
print(matches[0])
PY
)"
[ -x "$exe" ] || exit 2
# The fresh executable owns its compiled source; later invocations need no .rs.
rm -f rust/tcl-spectcl/tests/zz_review_probe.rs
failed=0
run_case() {
    local filter="$1" budget="$2" size="${3:-}"
    printf '\nCASE %s size=%s budget=%ss %s\n' "$filter" "$size" "$budget" "$(date -u)" >> "$log"
    ZZ_SCALE_P="$size" timeout "$budget" "$exe" "$filter" --nocapture --test-threads=1 >> "$log" 2>&1
    local status=$?
    printf 'CASE_EXIT %s size=%s exit=%s %s\n' "$filter" "$size" "$status" "$(date -u)" >> "$log"
    if [ "$status" -ne 0 ]; then failed=1; fi
}
run_case seam_ 240
run_case n17_live_hook_effect_control 300
run_case completed_world_transport_before_and_after_core_surface_seed 360
run_case z_braced_indirect_parser_and_scalar_parentheses 180
if rg -q 'R1_TRANSPORT seeded=true .* puts-call: .*analysis=true' "$log"; then
    run_case rename_with_registry 1200
else
    echo 'R1 positive puts-call control still lacks a completed world; post-fix matrix remains blocked.' >> "$log"
fi
# Incoming compiler changes justify a fresh lens series; keep analysis separate.
run_case scaling_chain_selected 240
for size in 25 50 100; do
    case "$size" in 25) budget=180;; 50) budget=300;; 100) budget=600;; esac
    run_case scaling_lens_selected "$budget" "$size"
done
if ! rg -q 'SCALE flat P=200 ' "$E/run6.log"; then
    run_case scaling_flat_selected 600 200
else
    echo 'Flat P=200 already complete in run6.log' >> "$log"
fi
cleanup
printf 'cleaned %s\n' "$(date -u)" >> "$log"
exit "$failed"

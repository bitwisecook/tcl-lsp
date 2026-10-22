#!/bin/bash
# Fresh latest-work semantic replay; no product/test files are retained.
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
tag="${1:-run8-final}"
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
for filter in rename_with_registry n01_identity n02_jim n03_variables n04_import_rename n05_forced_import n05_control_plain_rename n06_collisions n06_sibling_profile_matrix n07_speculative_var n08_inliner n08_unit_inference n09_compiler_facts n10_classes n11_packages n12_incremental n13_source_env n14_semantic_tokens n15_minify n16_providers n17_template_registration n17_live_hook n17_control_mylib seam_; do
    printf '\nCASE %s %s\n' "$filter" "$(date -u)" >> "$log"
    budget=600
    case "$filter" in rename_with_registry|n15_minify) budget=1200;; esac
    timeout "$budget" "$exe" "$filter" --nocapture --test-threads=1 >> "$log" 2>&1
    status=$?
    printf 'CASE_EXIT %s exit=%s %s\n' "$filter" "$status" "$(date -u)" >> "$log"
    if [ "$status" -ne 0 ]; then failed=1; fi
done
cleanup
# Repeat the user's named existing fold control on this same checkout.
{ date -u; git rev-parse HEAD; sha256sum rust/tcl-compiler/src/const_subst.rs; df -h /; } >> "$log"
cargo test --locked -p tcl-spectcl --test pack_source_e2e a_pack_file_folds_a_call_site_in_the_optimiser -- --nocapture --test-threads=1 >> "$log" 2>&1
status=$?
rm -f "$target_dir"/debug/deps/pack_source_e2e-*
printf 'E2E_EXIT=%s\n' "$status" >> "$log"
if [ "$status" -ne 0 ]; then failed=1; fi
printf 'cleaned %s\n' "$(date -u)" >> "$log"
exit "$failed"

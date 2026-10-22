#!/bin/bash
# Full review replay; an isolated checkout and the pinned oracle environment are required.
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
tag="${1:-run6}"
[[ "$tag" =~ ^run[0-9]+[a-z0-9-]*$ ]] || exit 2
log="$E/$tag.log"
export ZZ_SCRATCH="$PWD/$E/out/$tag"
if [ -e "$log" ] || [ -e "$ZZ_SCRATCH" ]; then
    echo 'Choose a fresh run tag; existing evidence is preserved.' >&2
    exit 2
fi
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH="$PWD/tmp/jimtcl-oracle/jimsh"
target_dir="${CARGO_TARGET_DIR:-target}"
cleanup() {
    rm -f rust/tcl-spectcl/tests/zz_review_probe.rs
    rm -f "$target_dir"/debug/deps/zz_review_probe-* "$target_dir"/debug/deps/pack_source_e2e-*
}
trap cleanup EXIT
mkdir -p "$ZZ_SCRATCH"
cp "$E/zz_review_probe.rs.txt" rust/tcl-spectcl/tests/zz_review_probe.rs
{
    date -u
    git rev-parse HEAD
    sha256sum rust/tcl-spectcl/tests/zz_review_probe.rs
    rustc --version
    df -h /
} > "$log"
timeout 3600 cargo test --locked -p tcl-spectcl --test zz_review_probe -- --nocapture --test-threads=1 >> "$log" 2>&1
main_rc=$?
echo "exit-main=$main_rc" >> "$log"
rm -f "$target_dir"/debug/deps/zz_review_probe-*
df -h / >> "$log"
timeout 2400 cargo test --locked -p tcl-spectcl --test pack_source_e2e a_pack_file_folds_a_call_site_in_the_optimiser -- --nocapture --test-threads=1 >> "$log" 2>&1
e2e_rc=$?
echo "exit-e2e=$e2e_rc" >> "$log"
cleanup
printf 'cleaned %s\n' "$(date -u)" >> "$log"
[ "$main_rc" -eq 0 ] && [ "$e2e_rc" -eq 0 ]

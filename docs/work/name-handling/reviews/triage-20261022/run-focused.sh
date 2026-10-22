#!/bin/bash
# Build/run/delete one selected observational probe on the current tree.
set -u
cd "$(git rev-parse --show-toplevel)"
E=docs/work/name-handling/reviews/triage-20261022
tag="${1:?provide a new run tag}"
filter="${2:?provide a test filter}"
log="$E/$tag.log"
[ ! -e "$log" ] || exit 2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH="$PWD/tmp/jimtcl-oracle/jimsh"
export ZZ_SCRATCH="$PWD/$E/out/$tag"
target_dir="${CARGO_TARGET_DIR:-target}"
cleanup() { rm -f rust/tcl-spectcl/tests/zz_review_probe.rs "$target_dir"/debug/deps/zz_review_probe-*; }
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
printf 'CASE %s %s\n' "$filter" "$(date -u)" >> "$log"
timeout 900 "$exe" "$filter" --nocapture --test-threads=1 >> "$log" 2>&1
status=$?
printf 'CASE_EXIT %s exit=%s %s\n' "$filter" "$status" "$(date -u)" >> "$log"
cleanup
exit "$status"

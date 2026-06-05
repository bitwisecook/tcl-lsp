#!/usr/bin/env bash
# Dump + compare every command-spec registry group (Python source vs Rust port).
# Artifacts land in tmp/registry-audit/. Run from the repo root.
#
# Fails loudly on any error (set -e) and refuses to publish empty dumps, so a
# broken build / import can never masquerade as a successful audit.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
OUT="tmp/registry-audit"
mkdir -p "$OUT"

# Always (re)build the Rust dumper. cargo is incremental, so this is a near
# no-op when up to date, and it guarantees the audit never compares against a
# stale binary after the Rust registry data or the example changes.
RUST_BIN="./target/debug/examples/dump_specs"
echo "building rust dumper..." >&2
(cd rust && cargo build -q --example dump_specs -p tcl-registry)

# NB: do not name this GROUPS — that is a bash built-in (user group IDs).
REG_GROUPS="tcl stdlib tcllib irules iapps tk expect sdc-base synopsys cadence xilinx quartus mentor"

for g in $REG_GROUPS; do
  python3 scripts/registry-audit/dump_python.py "$g" >"$OUT/$g.python.jsonl" 2>"$OUT/$g.python.err"
  "$RUST_BIN" "$g" >"$OUT/$g.rust.jsonl" 2>"$OUT/$g.rust.err"
  # A dump that exits 0 but produced nothing is still bad data — refuse it.
  if [ ! -s "$OUT/$g.python.jsonl" ] || [ ! -s "$OUT/$g.rust.jsonl" ]; then
    echo "ERROR: empty dump for group '$g' (see $OUT/$g.python.err / $OUT/$g.rust.err)" >&2
    exit 1
  fi
  python3 scripts/registry-audit/compare.py "$g" \
    "$OUT/$g.python.jsonl" "$OUT/$g.rust.jsonl" \
    --json "$OUT/$g.summary.json" >"$OUT/$g.report.txt" 2>>"$OUT/$g.python.err"
  echo "done: $g"
done

# BIG-IP object registry (GAP-e) — separate object-spec schema, audited
# by its own dumper/compare. Non-fatal so a transient diff is reported,
# not silently swallowed.
"$RUST_BIN" bigip >"$OUT/bigip.rust.jsonl" 2>"$OUT/bigip.rust.err"
python3 scripts/registry-audit/audit_bigip.py >"$OUT/bigip.report.txt" 2>&1 \
  && echo "done: bigip (parity OK)" \
  || echo "done: bigip (DIFFERENCES — see $OUT/bigip.report.txt)" >&2

# ---------------------------------------------------------------------------
# Authoritative completeness gate. The three audits below are the standing
# oracle for "have we slipped?": each asserts the Rust port carries at least
# as much data as the Python source of truth and exits non-zero on any
# deficiency. Running them here means a single `bash run_all.sh` fully checks
# the registries. Each is run with `|| FAIL=1` so all three always report,
# and the script exits non-zero if any found a gap.
echo >&2
echo "=== completeness gates ===" >&2
FAIL=0

echo "[1/3] count audit (every entry >= Python)..." >&2
python3 scripts/registry-audit/audit_completeness.py >"$OUT/audit_completeness.txt" 2>&1 \
  || FAIL=1
tail -1 "$OUT/audit_completeness.txt" >&2

echo "[2/3] deep content audit (field-by-field Rust >= Python)..." >&2
python3 scripts/registry-audit/audit_deep.py >"$OUT/audit_deep.txt" 2>&1 \
  || FAIL=1
tail -1 "$OUT/audit_deep.txt" >&2

echo "[3/3] bigip object-registry full-field parity..." >&2
python3 scripts/registry-audit/audit_bigip.py >"$OUT/bigip.report.txt" 2>&1 \
  || FAIL=1
tail -1 "$OUT/bigip.report.txt" >&2

if [ "$FAIL" -ne 0 ]; then
  echo >&2
  echo "REGISTRY AUDIT: deficiencies found — see $OUT/{audit_completeness,audit_deep,bigip.report}.txt" >&2
  exit 1
fi
echo >&2
echo "REGISTRY AUDIT: ✅ all registries complete (Rust >= Python on every field)" >&2

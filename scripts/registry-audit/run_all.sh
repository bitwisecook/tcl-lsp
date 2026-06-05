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

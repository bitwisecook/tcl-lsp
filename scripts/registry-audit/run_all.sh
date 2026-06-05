#!/usr/bin/env bash
# Dump + compare every command-spec registry group (Python source vs Rust port).
# Artifacts land in tmp/registry-audit/. Run from the repo root.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
OUT="tmp/registry-audit"
mkdir -p "$OUT"

RUST_BIN="./target/debug/examples/dump_specs"
if [ ! -x "$RUST_BIN" ]; then
  echo "building rust dumper..." >&2
  (cd rust && cargo build -q --example dump_specs -p tcl-registry)
fi

# NB: do not name this GROUPS — that is a bash built-in (user group IDs).
REG_GROUPS="tcl stdlib tcllib irules iapps tk expect sdc-base synopsys cadence xilinx quartus mentor"

for g in $REG_GROUPS; do
  python3 scripts/registry-audit/dump_python.py "$g" >"$OUT/$g.python.jsonl" 2>"$OUT/$g.python.err"
  "$RUST_BIN" "$g" >"$OUT/$g.rust.jsonl" 2>"$OUT/$g.rust.err"
  python3 scripts/registry-audit/compare.py "$g" \
    "$OUT/$g.python.jsonl" "$OUT/$g.rust.jsonl" \
    --json "$OUT/$g.summary.json" >"$OUT/$g.report.txt" 2>>"$OUT/$g.python.err"
  echo "done: $g"
done

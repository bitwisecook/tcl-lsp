#!/usr/bin/env python3
"""GAP-e: audit the Rust BigIP object-registry port against the Python
source of truth (the reconciled `OBJECT_SPECS`).

Dumps both sides with the same normalised schema (kind / module /
object_types / n_header_types / n_properties / property names+kinds) and
reports name parity + per-kind property parity.

Usage: python3 scripts/registry-audit/audit_bigip.py
       (build the Rust dumper first: cargo build -q --example dump_specs)
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from core.bigip.registry.specs import OBJECT_SPECS  # noqa: E402

RUST_BIN = ROOT / "target/debug/examples/dump_specs"


def prop_sig(p) -> str:
    """Full-field signature mirroring the Rust `bigip_prop_sig`."""
    def numfmt(v):
        if v is None:
            return ""
        return str(int(v)) if float(v).is_integer() else repr(v)
    shape = p.shape_kind or ""
    secs = ",".join(sorted(p.in_sections or ()))
    enums = ",".join(sorted(p.enum_values or ()))
    refs = ",".join(sorted(p.references or ()))
    flags = ",".join(sorted(p.usage_flags or ()))
    ops = ",".join(sorted(p.list_operators or ()))
    block = ";".join(sorted(prop_sig(c) for c in (p.block or ())))
    return (f"{p.name}~{p.value_type}~{int(p.required)}~{int(p.repeated)}~{int(p.allow_none)}~"
            f"{secs}~{numfmt(p.min_value)}~{numfmt(p.max_value)}~{p.pattern}~{refs}~"
            f"{p.description}~{shape}~{p.default or ''}~{enums}~{flags}~{ops}~[{block}]")


def py_record(o) -> dict:
    ks = o.kind_spec
    return {
        "kind": ks.kind,
        "module": ks.module,
        "object_types": sorted(ks.object_types or ()),
        "n_header_types": len(o.header_types or ()),
        "n_properties": len(o.properties or ()),
        "properties": sorted(p.name for p in o.properties),
        # Full-field per-property signature, sorted multiset (duplicates
        # kept — the bgp object repeats property names with distinct data).
        "property_pairs": sorted(prop_sig(p) for p in o.properties),
    }


def main() -> None:
    py = {o.kind_spec.kind: py_record(o) for o in OBJECT_SPECS}
    out = subprocess.run([str(RUST_BIN), "bigip"], capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit(f"rust dumper failed: {out.stderr}")
    ru = {}
    for line in out.stdout.splitlines():
        d = json.loads(line)
        ru[d["kind"]] = d

    pk, rk = set(py), set(ru)
    missing = sorted(pk - rk)
    extra = sorted(rk - pk)
    print(f"BigIP object kinds: python={len(pk)} rust={len(rk)}")
    print(f"  missing in rust: {len(missing)}  extra in rust: {len(extra)}")
    if missing:
        print("  missing sample:", missing[:8])
    if extra:
        print("  extra sample:", extra[:8])

    # Per-kind property parity over the shared set.
    shared = sorted(pk & rk)
    nprop_diff = []
    propname_diff = []
    kind_diff = []
    module_diff = []
    for k in shared:
        p, r = py[k], ru[k]
        if p["n_properties"] != r["n_properties"]:
            nprop_diff.append(k)
        if p["properties"] != r["properties"]:
            propname_diff.append(k)
        if p["module"] != r["module"]:
            module_diff.append(k)
        # value_kind per property (order-independent multiset of pairs)
        if p["property_pairs"] != r["property_pairs"]:
            kind_diff.append(k)
    print(f"  shared kinds: {len(shared)}")
    print(f"    property-count mismatches: {len(nprop_diff)} {nprop_diff[:5]}")
    print(f"    property-name-set mismatches: {len(propname_diff)} {propname_diff[:5]}")
    print(f"    property value-kind mismatches: {len(kind_diff)} {kind_diff[:5]}")
    print(f"    module mismatches: {len(module_diff)} {module_diff[:5]}")

    ok = not missing and not extra and not nprop_diff and not propname_diff and not kind_diff and not module_diff
    print("\nVERDICT:", "OK ✅ full parity" if ok else "DIFFERENCES ❌")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Compare normalised registry dumps (Python source vs Rust port).

Reads two JSONL files produced by ``dump_python.py`` and the Rust
``dump_specs`` example, then reports:

  * name parity (missing in Rust / extra in Rust / duplicates)
  * a completeness matrix (per data dimension: how many commands carry
    that data on each side)
  * field-value mismatches for commands present on both sides

Emits a human-readable report to stdout and, with ``--json FILE``, a
machine summary for embedding into the audit doc.

Usage: compare.py <group> <python.jsonl> <rust.jsonl> [--json out.json]
"""

from __future__ import annotations

import json
import sys

# Fields whose *value* we compare directly for common commands.
VALUE_FIELDS = [
    "dialects_all",
    "dialects",
    "arity_min",
    "arity_max",
    "hover",
    "summary",
    "synopsis",
    "source_is_url",
    "examples",
    "return_value",
    "n_forms",
    "options",
    "n_subcommands",
    "subcommands",
    "n_side_effects",
    "event_profiles",
    "event_also_in",
    "event_requires_any",
    "excluded_events",
    "required_package",
    "return_type",
    "n_arg_types",
    "n_arg_roles",
    "body_kind",
]

# "Has data" predicates for the completeness matrix.
DIMENSIONS: dict[str, callable] = {
    "hover": lambda r: bool(r.get("hover")),
    "hover_synopsis": lambda r: bool(r.get("synopsis")),
    "hover_source_url": lambda r: bool(r.get("source_is_url")),
    "hover_examples": lambda r: bool(r.get("examples")),
    "hover_return_value": lambda r: bool(r.get("return_value")),
    "forms": lambda r: r.get("n_forms", 0) > 0,
    "options": lambda r: bool(r.get("options")),
    "subcommands": lambda r: r.get("n_subcommands", 0) > 0,
    "side_effects": lambda r: r.get("n_side_effects", 0) > 0,
    "event_requires": lambda r: bool(r.get("event_requires_any")),
    "event_profiles": lambda r: bool(r.get("event_profiles")),
    "excluded_events": lambda r: bool(r.get("excluded_events")),
    "required_package": lambda r: r.get("required_package") is not None,
    "return_type": lambda r: r.get("return_type") is not None,
    "arg_types": lambda r: r.get("n_arg_types", 0) > 0,
    "arg_roles": lambda r: r.get("n_arg_roles", 0) > 0,
    "arity_bounded": lambda r: (r.get("arity_min", 0) > 0) or (r.get("arity_max") is not None),
    "lowering_hook": lambda r: bool(r.get("has_lowering")),
    "codegen_hook": lambda r: bool(r.get("has_codegen")),
    "const_fold": lambda r: bool(r.get("has_const_fold")),
    "traits": lambda r: bool(r.get("has_traits")),
}


def _canon_body_kind(v: str | None) -> str:
    s = (v or "").lower()
    return {"plain": "inline"}.get(s, s)


def canon(field: str, value):
    if field == "body_kind":
        return _canon_body_kind(value)
    return value


def load(path: str) -> tuple[dict[str, dict], list[str]]:
    by_name: dict[str, dict] = {}
    dups: list[str] = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            n = rec["name"]
            if n in by_name:
                dups.append(n)
            by_name[n] = rec
    return by_name, dups


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    json_out = None
    if "--json" in sys.argv:
        json_out = sys.argv[sys.argv.index("--json") + 1]
    if len(args) < 3:
        sys.stderr.write("usage: compare.py <group> <python.jsonl> <rust.jsonl> [--json out]\n")
        return 2
    group, py_path, rs_path = args[0], args[1], args[2]

    py, py_dups = load(py_path)
    rs, rs_dups = load(rs_path)

    py_names = set(py)
    rs_names = set(rs)
    missing = sorted(py_names - rs_names)  # in Python, absent in Rust
    extra = sorted(rs_names - py_names)  # in Rust, absent in Python
    common = sorted(py_names & rs_names)

    # completeness matrix
    matrix = {}
    for dim, pred in DIMENSIONS.items():
        pc = sum(1 for r in py.values() if pred(r))
        rc = sum(1 for r in rs.values() if pred(r))
        matrix[dim] = {"python": pc, "rust": rc, "delta": rc - pc}

    # field mismatches over common commands
    field_mismatch: dict[str, dict] = {}
    for f in VALUE_FIELDS:
        diffs = []
        for n in common:
            pv = canon(f, py[n].get(f))
            rv = canon(f, rs[n].get(f))
            if pv != rv:
                diffs.append(n)
        if diffs:
            field_mismatch[f] = {"count": len(diffs), "examples": diffs[:8]}

    summary = {
        "group": group,
        "python_count": len(py),
        "rust_count": len(rs),
        "missing_in_rust": missing,
        "extra_in_rust": extra,
        "common": len(common),
        "python_dups": sorted(set(py_dups)),
        "rust_dups": sorted(set(rs_dups)),
        "completeness": matrix,
        "field_mismatch": field_mismatch,
    }

    # human-readable
    print(f"### group: {group}")
    print(f"  python specs: {len(py)}   rust specs: {len(rs)}   common: {len(common)}")
    if py_dups or rs_dups:
        print(f"  duplicate names  python={sorted(set(py_dups))}  rust={sorted(set(rs_dups))}")
    print(f"  MISSING in rust ({len(missing)}): {missing[:20]}{' ...' if len(missing) > 20 else ''}")
    print(f"  EXTRA in rust   ({len(extra)}): {extra[:20]}{' ...' if len(extra) > 20 else ''}")
    print("  completeness (commands carrying each dimension):")
    print(f"    {'dimension':22} {'python':>7} {'rust':>7} {'delta':>7}")
    for dim, v in matrix.items():
        flag = "  <-- GAP" if v["delta"] < 0 else ""
        print(f"    {dim:22} {v['python']:>7} {v['rust']:>7} {v['delta']:>+7}{flag}")
    print("  field-value mismatches over common commands:")
    if not field_mismatch:
        print("    (none)")
    for f, v in sorted(field_mismatch.items(), key=lambda kv: -kv[1]["count"]):
        print(f"    {f:22} {v['count']:>5}   e.g. {v['examples'][:5]}")

    if json_out:
        with open(json_out, "w") as fh:
            json.dump(summary, fh, indent=2, sort_keys=True)
        sys.stderr.write(f"[compare] wrote {json_out}\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

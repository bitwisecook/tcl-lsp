#!/usr/bin/env python3
"""GAP-e stage 3: generate the Rust BigIP object-registry data from the
reconciled Python `OBJECT_SPECS` (the canonical source of truth).

Emits `rust/tcl-registry/src/bigip/data/<letter>.rs` (one file per
kind-name initial, each a `pub static SPECS: &[BigipObjectSpec]`) plus
`data/mod.rs` aggregating them via `all_specs()`.
"""

from __future__ import annotations

import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from core.bigip.registry.specs import OBJECT_SPECS  # noqa: E402

OUT = ROOT / "rust/tcl-registry/src/bigip/data"

_VK = {
    "string": "String", "integer": "Integer", "float": "Float",
    "boolean": "Boolean", "enum": "Enum", "reference": "Reference",
    "list": "List", "block": "Block", "unknown": "Unknown",
    "ip-address": "IpAddress", "endpoint": "Endpoint", "object": "Object",
}


def rs(s: str) -> str:
    return (
        '"'
        + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\t", "\\t").replace("\r", "\\r")
        + '"'
    )


def vk(s: str) -> str:
    return f"ValueKind::{_VK.get(s, 'Unknown')}"


def slice_str(items) -> str:
    return "&[" + ", ".join(rs(x) for x in items) + "]"


def opt_str(v) -> str:
    return f"Some({rs(v)})" if v is not None else "None"


def num(v) -> str:
    if v is None:
        return "None"
    # Emit a plain f64 literal.
    f = float(v)
    return f"Some({f!r}f64)" if f != int(f) else f"Some({int(f)}f64)"


def prop_literal(p, indent: str) -> str:
    inner = indent + "    "
    lines = [f"{indent}BigipPropertySpec {{"]
    lines.append(f"{inner}name: {rs(p.name)},")
    lines.append(f"{inner}value_type: {vk(p.value_type)},")
    if p.in_sections:
        lines.append(f"{inner}in_sections: {slice_str(p.in_sections)},")
    if p.required:
        lines.append(f"{inner}required: true,")
    if p.repeated:
        lines.append(f"{inner}repeated: true,")
    if p.allow_none:
        lines.append(f"{inner}allow_none: true,")
    if p.enum_values:
        lines.append(f"{inner}enum_values: {slice_str(p.enum_values)},")
    if p.min_value is not None:
        lines.append(f"{inner}min_value: {num(p.min_value)},")
    if p.max_value is not None:
        lines.append(f"{inner}max_value: {num(p.max_value)},")
    if p.pattern:
        lines.append(f"{inner}pattern: {rs(p.pattern)},")
    if p.references:
        lines.append(f"{inner}references: {slice_str(p.references)},")
    if p.description:
        lines.append(f"{inner}description: {rs(p.description)},")
    if p.shape_kind:
        lines.append(f"{inner}shape_kind: Some({vk(p.shape_kind)}),")
    if p.default is not None:
        lines.append(f"{inner}default: {opt_str(p.default)},")
    if p.usage_flags:
        lines.append(f"{inner}usage_flags: {slice_str(sorted(p.usage_flags))},")
    if p.list_operators:
        lines.append(f"{inner}list_operators: {slice_str(sorted(p.list_operators))},")
    if p.block:
        body = "\n".join(prop_literal(c, inner + "    ") for c in p.block)
        lines.append(f"{inner}block: &[\n{body}\n{inner}],")
    lines.append(f"{inner}..BigipPropertySpec::DEFAULT")
    lines.append(f"{indent}}},")
    return "\n".join(lines)


def kind_literal(ks, indent: str) -> str:
    inner = indent + "    "
    return (
        f"{indent}BigipObjectKindSpec {{\n"
        f"{inner}kind: {rs(ks.kind)},\n"
        f"{inner}table_name: {opt_str(ks.table_name)},\n"
        f"{inner}resolver_name: {opt_str(ks.resolver_name)},\n"
        f"{inner}module: {opt_str(ks.module)},\n"
        f"{inner}object_types: {slice_str(ks.object_types)},\n"
        f"{indent}}}"
    )


def spec_literal(o) -> str:
    headers = "&[" + ", ".join(f"({rs(m)}, {rs(t)})" for m, t in o.header_types) + "]"
    props = "\n".join(prop_literal(p, "            ") for p in o.properties)
    return (
        "    BigipObjectSpec {\n"
        f"        kind_spec: {kind_literal(o.kind_spec, '        ')},\n"
        f"        header_types: {headers},\n"
        f"        properties: &[\n{props}\n        ],\n"
        "    },"
    )


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for old in OUT.glob("*.rs"):
        old.unlink()

    buckets: dict[str, list] = defaultdict(list)
    for o in sorted(OBJECT_SPECS, key=lambda s: s.kind_spec.kind):
        c = o.kind_spec.kind[0].lower()
        buckets[c if c.isalpha() else "_"].append(o)

    header = (
        "//! Generated BIG-IP object specs. DO NOT EDIT — produced by\n"
        "//! `scripts/registry-audit/gen_bigip_rust.py` from the reconciled\n"
        "//! Python `OBJECT_SPECS` (canonical `origin/main` baseline).\n"
        "// Some buckets hold property-less kinds, so not every imported type\n"
        "// is used in every file; large tmsh bounds appear as bare f64 literals.\n"
        "#![allow(unused_imports, clippy::unreadable_literal)]\n"
        "use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};\n\n"
    )
    letters = sorted(buckets)
    for c in letters:
        body = "\n".join(spec_literal(o) for o in buckets[c])
        text = header + f"pub static SPECS: &[BigipObjectSpec] = &[\n{body}\n];\n"
        (OUT / f"{c}.rs").write_text(text)

    mods = "\n".join(f"mod {c};" for c in letters)
    extend = "\n".join(f"    v.extend({c}::SPECS.iter());" for c in letters)
    modrs = (
        "//! Generated BIG-IP object-spec data modules (one per kind-name\n"
        "//! initial). Aggregated by [`all_specs`].\n"
        "use super::BigipObjectSpec;\n\n"
        f"{mods}\n\n"
        "/// All generated BIG-IP object specs, in kind-name order.\n"
        "#[must_use]\n"
        "pub fn all_specs() -> Vec<&'static BigipObjectSpec> {\n"
        "    let mut v: Vec<&'static BigipObjectSpec> = Vec::new();\n"
        f"{extend}\n"
        "    v\n"
        "}\n"
    )
    (OUT / "mod.rs").write_text(modrs)
    total = sum(len(v) for v in buckets.values())
    print(f"generated {len(letters)} data files, {total} object specs")


if __name__ == "__main__":
    main()

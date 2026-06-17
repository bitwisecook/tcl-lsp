#!/usr/bin/env python3
"""Generate the Rust BIG-IP object-spec data from the Python ``OBJECT_SPECS``.

Emits ``rust/tcl-registry/src/bigip/data/<initial>.rs`` (one file per kind-name
initial, specs in ``OBJECT_SPECS`` order) plus the aggregating ``mod.rs``. The
Rust ``BigipObjectSpec`` / ``BigipPropertySpec`` structs mirror the Python
dataclasses 1:1; this is the single source of truth so the generated data stays
reconciled with the reconciled ``OBJECT_SPECS`` baseline.

Run from the repo root:  ``python scripts/registry-audit/gen_bigip_rust.py``
"""

from __future__ import annotations

import pathlib

from dialects.f5.bigip.registry.data import OBJECT_SPECS

# Python ValueKind wire value -> Rust ValueKind variant.
_VK = {
    "string": "String",
    "integer": "Integer",
    "float": "Float",
    "boolean": "Boolean",
    "enum": "Enum",
    "reference": "Reference",
    "list": "List",
    "block": "Block",
    "unknown": "Unknown",
    "ip-address": "IpAddress",
    "endpoint": "Endpoint",
    "object": "Object",
}

_OUT_DIR = pathlib.Path("rust/tcl-registry/src/bigip/data")

_HEADER = """\
//! Generated BIG-IP object specs. DO NOT EDIT — produced by
//! `scripts/registry-audit/gen_bigip_rust.py` from the reconciled
//! Python `OBJECT_SPECS` (canonical `origin/main` baseline).
// Some buckets hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};
"""


def _rstr(s: str) -> str:
    """A Rust string literal for *s*."""
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def _str_slice(items) -> str:
    return "&[" + ", ".join(_rstr(x) for x in items) + "]"


def _vk(value: str) -> str:
    return f"ValueKind::{_VK[value]}"


def _opt_str(v) -> str:
    return "None" if v is None else f"Some({_rstr(v)})"


def _opt_f64(v) -> str:
    if v is None:
        return "None"
    if isinstance(v, float) and v.is_integer():
        v = int(v)
    return f"Some({v}f64)"


def _vk_or_none(shape_kind: str) -> str:
    return f"Some({_vk(shape_kind)})" if shape_kind else "None"


def _emit_prop(p, indent: int) -> list[str]:
    pad = " " * indent
    fp = " " * (indent + 4)
    out = [f"{pad}BigipPropertySpec {{", f"{fp}name: {_rstr(p.name)},"]
    out.append(f"{fp}value_type: {_vk(p.value_kind.value)},")
    if p.in_sections:
        out.append(f"{fp}in_sections: {_str_slice(p.in_sections)},")
    if p.required:
        out.append(f"{fp}required: true,")
    if p.repeated:
        out.append(f"{fp}repeated: true,")
    if p.allow_none:
        out.append(f"{fp}allow_none: true,")
    if p.enum_values:
        out.append(f"{fp}enum_values: {_str_slice(p.enum_values)},")
    if p.min_value is not None:
        out.append(f"{fp}min_value: {_opt_f64(p.min_value)},")
    if p.max_value is not None:
        out.append(f"{fp}max_value: {_opt_f64(p.max_value)},")
    if p.pattern:
        out.append(f"{fp}pattern: {_rstr(p.pattern)},")
    if p.references:
        out.append(f"{fp}references: {_str_slice(p.references)},")
    if p.description:
        out.append(f"{fp}description: {_rstr(p.description)},")
    if p.shape_kind:
        out.append(f"{fp}shape_kind: {_vk_or_none(p.shape_kind)},")
    if p.default is not None:
        out.append(f"{fp}default: {_opt_str(p.default)},")
    if p.usage_flags:
        # frozenset -> sorted slice (the old generator sorted these).
        out.append(f"{fp}usage_flags: {_str_slice(sorted(p.usage_flags))},")
    if p.list_operators:
        out.append(f"{fp}list_operators: {_str_slice(sorted(p.list_operators))},")
    if p.block:
        out.append(f"{fp}block: &[")
        for sub in p.block:
            out.extend(_emit_prop(sub, indent + 8))
        out.append(f"{fp}],")
    out.append(f"{fp}..BigipPropertySpec::DEFAULT")
    out.append(f"{pad}}},")
    return out


def _emit_spec(s) -> list[str]:
    ks = s.kind_spec
    out = ["    BigipObjectSpec {"]
    out.append("        kind_spec: BigipObjectKindSpec {")
    out.append(f"            kind: {_rstr(ks.kind)},")
    out.append(f"            table_name: {_opt_str(ks.table_name)},")
    out.append(f"            resolver_name: {_opt_str(ks.resolver_name)},")
    out.append(f"            module: {_opt_str(ks.module)},")
    out.append(f"            object_types: {_str_slice(ks.object_types)},")
    out.append("        },")
    headers = ", ".join(f"({_rstr(m)}, {_rstr(o)})" for (m, o) in s.header_types)
    out.append(f"        header_types: &[{headers}],")
    if s.properties:
        out.append("        properties: &[")
        for p in s.properties:
            out.extend(_emit_prop(p, 12))
        out.append("        ],")
    else:
        out.append("        properties: &[],")
    out.append("    },")
    return out


def main() -> None:
    specs = list(OBJECT_SPECS)
    buckets: dict[str, list] = {}
    for s in specs:
        buckets.setdefault(s.kind_spec.kind[0], []).append(s)

    # Remove any stale per-initial files so a dropped bucket disappears.
    for old in _OUT_DIR.glob("*.rs"):
        if old.name != "mod.rs":
            old.unlink()

    letters = sorted(buckets)
    for letter in letters:
        lines = [_HEADER, "", "pub static SPECS: &[BigipObjectSpec] = &["]
        for s in buckets[letter]:
            lines.extend(_emit_spec(s))
        lines.append("];")
        (_OUT_DIR / f"{letter}.rs").write_text("\n".join(lines) + "\n")

    # mod.rs aggregating every bucket in kind-name (file) order.
    mod = [
        "//! Generated BIG-IP object-spec data modules (one per kind-name",
        "//! initial). Aggregated by [`all_specs`].",
        "use super::BigipObjectSpec;",
        "",
    ]
    for letter in letters:
        mod.append(f"mod {letter};")
    mod += [
        "",
        "/// All generated BIG-IP object specs, in kind-name order.",
        "#[must_use]",
        "pub fn all_specs() -> Vec<&'static BigipObjectSpec> {",
        "    let mut v: Vec<&'static BigipObjectSpec> = Vec::new();",
    ]
    for letter in letters:
        mod.append(f"    v.extend({letter}::SPECS.iter());")
    mod += ["    v", "}", ""]
    (_OUT_DIR / "mod.rs").write_text("\n".join(mod) + "\n")

    total_props = sum(len(s.properties) for s in specs)
    print(
        f"generated {len(specs)} specs across {len(letters)} buckets "
        f"({total_props} top-level properties)"
    )


if __name__ == "__main__":
    main()

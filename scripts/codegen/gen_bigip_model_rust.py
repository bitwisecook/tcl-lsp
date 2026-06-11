#!/usr/bin/env python3
"""Generate Rust struct definitions for the BIG-IP object model.

Introspects the Python dataclasses under ``dialects/f5/bigip/model/`` and
emits faithful Rust structs (field names, types, and Default impls
matching the Python defaults) into ``rust/tcl-bigip/src/model/gen/``.

This is the *model* half of the faithful port — the per-kind parsers and
the BigipConfig container live elsewhere. Run from the repo root:

    python3 scripts/codegen/gen_bigip_model_rust.py
"""

from __future__ import annotations

import dataclasses as dc
import importlib
import re
from pathlib import Path

MODEL_MODULES = [
    "_ltm",
    "_gtm",
    "_security",
    "_net",
    "_sys",
    "_auth",
    "_apm",
    "_cm",
    "_pem",
]

# Python value-type / enum names that already exist on the Rust side.
VALUE_TYPES = {
    "BigipList": "crate::value::BigipList",
    "Address": "crate::value::Address",
    "IPAddress": "crate::value::IPAddress",
    "FQDN": "crate::value::FQDN",
    "Network": "crate::value::Network",
    "Destination": "crate::value::Destination",
}
ENUMS = {"DataGroupType", "ProfileType"}
OUT_DIR = Path("rust/tcl-bigip/src/model/gen")


def collect_classes() -> dict[str, tuple[str, type]]:
    """Return {class_name: (module_suffix, cls)} for every model dataclass."""
    found: dict[str, tuple[str, type]] = {}
    for mod in MODEL_MODULES:
        m = importlib.import_module(f"dialects.f5.bigip.model.{mod}")
        for name, obj in vars(m).items():
            if dc.is_dataclass(obj) and obj.__module__ == m.__name__:
                found[name] = (mod.lstrip("_"), obj)
    return found


def base_type(ann: str) -> tuple[str, bool]:
    """Return (inner_type, optional) from a string annotation."""
    s = str(ann).replace("typing.", "").strip().strip("'")
    optional = False
    if s.endswith("| None"):
        optional = True
        s = s[: -len("| None")].strip().strip("'")
    return s, optional


def rust_type(ann: str) -> str | None:
    """Map a Python field annotation to a Rust type, or None to skip."""
    inner, optional = base_type(ann)
    rust = _rust_inner(inner)
    if rust is None:
        return None
    return f"Option<{rust}>" if optional else rust


def _rust_inner(inner: str) -> str | None:
    if inner == "str":
        return "String"
    if inner == "int":
        return "i64"
    if inner == "bool":
        return "bool"
    if inner == "Range":
        return "crate::range::Range"
    if inner == "tuple[str, ...]":
        return "Vec<String>"
    if inner == "dict[str, tuple[int, int]]":
        return "std::collections::HashMap<String, (usize, usize)>"
    if inner in VALUE_TYPES:
        return VALUE_TYPES[inner]
    if inner in ENUMS:
        return f"crate::model::{inner}"
    m = re.fullmatch(r"tuple\[(Bigip[A-Za-z0-9_]+), \.\.\.\]", inner)
    if m:
        return f"Vec<{m.group(1)}>"
    # dict[str, Bigip...] only appears on BigipConfig — skip here.
    if inner.startswith("dict[str, Bigip"):
        return None
    if inner.startswith("Bigip"):
        return inner
    return None


def rust_default(field: dc.Field, rust_ty: str) -> str:
    """Return the Rust default expression matching the Python default."""
    if field.default is not dc.MISSING and field.default is not None:
        d = field.default
        if isinstance(d, bool):
            return "true" if d else "false"
        if isinstance(d, int):
            return str(d)
        if isinstance(d, str):
            return "String::new()" if d == "" else f"{d!r}.to_owned()".replace("'", '"')
    if rust_ty.startswith("Option<"):
        return "None"
    if rust_ty == "String":
        return "String::new()"
    if rust_ty == "i64":
        return "0"
    if rust_ty == "bool":
        return "false"
    if rust_ty.startswith("Vec<"):
        return "Vec::new()"
    if rust_ty.startswith("std::collections::HashMap"):
        return "std::collections::HashMap::new()"
    if rust_ty in ENUMS:
        return f"{rust_ty}::default()"
    return "Default::default()"


def snake_doc(name: str) -> str:
    return f"/// Mirrors Python `{name}`."


def gen_struct(name: str, cls: type) -> str:
    lines = [snake_doc(name), "#[derive(Debug, Clone, PartialEq)]", f"pub struct {name} {{"]
    defaults: list[tuple[str, str]] = []
    for f in dc.fields(cls):
        rty = rust_type(f.type)
        if rty is None:
            continue
        lines.append(f"    /// `{f.name}`")
        lines.append(f"    pub {f.name}: {rty},")
        defaults.append((f.name, rust_default(f, rty)))
    lines.append("}")
    lines.append("")
    lines.append(f"impl Default for {name} {{")
    lines.append("    fn default() -> Self {")
    lines.append("        Self {")
    for fname, dexpr in defaults:
        lines.append(f"            {fname}: {dexpr},")
    lines.append("        }")
    lines.append("    }")
    lines.append("}")
    return "\n".join(lines)


def snake(name: str) -> str:
    s = re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()
    return s


SCALAR_SKIP = {"name", "full_path", "range", "description", "state", "module"}


def gen_parser(name: str, cls: type) -> str:
    """Emit a faithful scalar parser for *cls* (structured fields left at
    their Default; correct for the scalar-only generable kinds)."""
    fn = f"parse_{snake(name)}"
    lines = [
        f"/// Scalar parser for `{name}` (generated). Structured fields stay",
        "/// at their `Default`; faithful for scalar-only kinds.",
        "#[must_use]",
        f"pub fn {fn}(full_path: &str, body: &str, range: crate::range::Range) -> {name} {{",
        "    let props = crate::parser::scalar::props_map(body);",
        f"    let mut obj = {name}::default();",
        "    let _ = &props;",
    ]
    has_fields = False
    for f in dc.fields(cls):
        rty = rust_type(f.type)
        if rty is None:
            continue
        nm = f.name
        key = nm.replace("_", "-")
        if nm == "name":
            lines.append("    obj.name = crate::parser::scalar::name_leaf(full_path);")
        elif nm == "full_path":
            lines.append("    obj.full_path = full_path.to_owned();")
        elif nm == "range":
            lines.append("    obj.range = Some(range);")
        elif nm == "description" and rty == "String":
            lines.append("    obj.description = crate::parser::scalar::description(&props);")
        elif nm == "state" and rty == "String":
            lines.append("    obj.state = crate::parser::scalar::state_flag(&props);")
        elif nm == "module":
            pass  # keep Default (constant module word)
        elif rty == "String":
            lines.append(f'    obj.{nm} = crate::parser::scalar::get_str(&props, "{key}");')
        elif rty == "i64":
            lines.append(f'    obj.{nm} = crate::parser::scalar::get_int(&props, "{key}");')
        elif rty == "bool":
            lines.append(f'    obj.{nm} = crate::parser::scalar::get_bool(&props, "{key}");')
        elif rty == "Vec<String>":
            lines.append(f'    obj.{nm} = crate::parser::scalar::list_field(&props, "{key}");')
        else:
            continue  # structured — bespoke parser fills it
        has_fields = True
    if not has_fields:
        lines.append("    let _ = (full_path, range);")
    lines.append("    obj")
    lines.append("}")
    return "\n".join(lines)


def main() -> None:
    classes = collect_classes()
    by_module: dict[str, list[str]] = {}
    for name, (mod, cls) in sorted(classes.items()):
        by_module.setdefault(mod, []).append(name)
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    mod_names = sorted(by_module)
    for mod in mod_names:
        out = [
            "// @generated by scripts/codegen/gen_bigip_model_rust.py — do not edit.",
            f"//! Generated BIG-IP `{mod}` model structs (port of"
            f" `dialects/f5/bigip/model/_{mod}.py`).",
            "",
            "#![allow(clippy::struct_excessive_bools)]",
            "#![allow(clippy::derivable_impls)]",
            "#![allow(clippy::default_trait_access)]",
            "#![allow(unused_imports)]",
            "",
            "use super::*;",
            "",
        ]
        for name in by_module[mod]:
            out.append(gen_struct(name, classes[name][1]))
            out.append("")
        (OUT_DIR / f"{mod}.rs").write_text("\n".join(out) + "\n")

    # gen/parsers.rs — a scalar parser per struct.
    parsers = [
        "// @generated by scripts/codegen/gen_bigip_model_rust.py — do not edit.",
        "//! Generated scalar per-kind parsers (port of the scalar-field half"
        " of `dialects/f5/bigip/parser/_parsers.py`).",
        "",
        "#![allow(clippy::needless_pass_by_value)]",
        "#![allow(clippy::assigning_clones)]",
        "#![allow(clippy::wildcard_imports)]",
        "#![allow(unused_variables)]",
        "",
        "use super::*;",
        "",
    ]
    for mod in mod_names:
        for name in by_module[mod]:
            parsers.append(gen_parser(name, classes[name][1]))
            parsers.append("")
    (OUT_DIR / "parsers.rs").write_text("\n".join(parsers) + "\n")

    # gen/object.rs — the ModelObject enum over every distinct placed kind.
    from dialects.f5.bigip.model._config import BigipConfig

    placed: list[str] = []
    seen: set[str] = set()
    for f in dc.fields(BigipConfig):
        m = re.search(r"dict\[str,\s*([A-Za-z0-9_]+)\]", str(f.type))
        if not m:
            continue
        struct = m.group(1)
        if "Minimal" in struct:
            struct = "BigipMinimalObject"
        if struct in seen:
            continue
        seen.add(struct)
        placed.append(struct)

    def variant(struct: str) -> str:
        return struct[len("Bigip") :] if struct.startswith("Bigip") else struct

    obj = [
        "// @generated by scripts/codegen/gen_bigip_model_rust.py — do not edit.",
        "//! Generated `ModelObject` enum over every placed BIG-IP kind.",
        "",
        "#![allow(clippy::large_enum_variant)]",
        "#![allow(clippy::wildcard_imports)]",
        "",
        "use super::*;",
        "use crate::model::{BigipGenericObject, BigipMinimalObject};",
        "",
        "/// A parsed BIG-IP object of any kind, as produced by the driver.",
        "#[derive(Debug, Clone, PartialEq)]",
        "pub enum ModelObject {",
    ]
    for struct in placed:
        obj.append(f"    /// `{struct}`.")
        obj.append(f"    {variant(struct)}({struct}),")
    obj.append("}")
    (OUT_DIR / "object.rs").write_text("\n".join(obj) + "\n")

    # gen/mod.rs re-exporting every generated struct + parsers + object.
    mod_rs = [
        "// @generated by scripts/codegen/gen_bigip_model_rust.py — do not edit.",
        "//! Generated BIG-IP model structs, grouped by tmsh module.",
        "",
    ]
    for mod in mod_names:
        mod_rs.append(f"mod {mod};")
    mod_rs.append("pub mod object;")
    mod_rs.append("pub mod parsers;")
    mod_rs.append("")
    for mod in mod_names:
        names = ", ".join(by_module[mod])
        mod_rs.append(f"pub use {mod}::{{{names}}};")
    mod_rs.append("pub use object::ModelObject;")
    (OUT_DIR / "mod.rs").write_text("\n".join(mod_rs) + "\n")

    total = sum(len(v) for v in by_module.values())
    print(
        f"generated {total} structs + {total} parsers + ModelObject"
        f" ({len(placed)} variants) into {OUT_DIR}"
    )


if __name__ == "__main__":
    main()

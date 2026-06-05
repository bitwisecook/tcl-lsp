#!/usr/bin/env python3
"""Generate individual Rust files for the genuinely-named tcl commands
missing from the Rust port (auto_* family, bgerror, pwd, nextto,
pkg_mkindex, tcl_findLibrary, tcl::build-info, pkg::create, filename,
http, memory), and wire them into `mod.rs`.

Name-parity reconcile (GAP-d), from the Python source of truth.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import load_specs  # noqa: E402

ANY = sys.maxsize
TCL_DIR = _REPO_ROOT / "rust/tcl-registry/src/commands/tcl"
MOD = TCL_DIR / "mod.rs"
_TYPE = {
    "INT": "Int", "STRING": "String", "LIST": "List", "DICT": "Dict",
    "BOOLEAN": "Boolean", "NUMERIC": "Numeric", "DOUBLE": "Double",
    "CHANNEL": "Channel",
}


def rust_str(s: str) -> str:
    return (
        '"'
        + (s or "").replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\t", "\\t")
        + '"'
    )


def arity_expr(ar) -> str:
    if ar is None:
        return "Arity::any()"
    lo, hi = ar.min, ar.max
    if hi >= ANY:
        return f"Arity::at_least({lo})"
    if lo == hi:
        return f"Arity::exact({lo})"
    return f"Arity::new({lo}, {hi})"


def module_name(cmd: str) -> str:
    return cmd.replace("::", "__").replace("-", "_").lower()


def file_text(spec) -> str:
    val = getattr(spec, "validation", None)
    arity = getattr(val, "arity", None) if val else None
    h = spec.hover
    lines = [
        f"//! `{spec.name}` command (name-parity reconcile, GAP-d).",
        "use crate::prelude::*;",
        "pub fn spec() -> CommandSpec {",
        "    CommandSpec {",
        f"        name: {rust_str(spec.name)},",
    ]
    if getattr(spec, "pure", False):
        lines.append("        traits: Traits::PURE,")
    lines.append(f"        arity: {arity_expr(arity)},")
    rt = spec.return_type
    if rt is not None and rt.name in _TYPE:
        lines.append(f"        return_type: Some(TclType::{_TYPE[rt.name]}),")
    if h is not None:
        syn = ", ".join(rust_str(s) for s in (h.synopsis or ()))
        lines.append(
            "        hover: Some(HoverSnippet::brief(\n"
            f"            {rust_str(h.summary or '')},\n"
            f"            &[{syn}],\n"
            f"            {rust_str(h.source or 'Tcl')},\n"
            "        )),"
        )
    lines += ["        ..CommandSpec::DEFAULT", "    }", "}", ""]
    return "\n".join(lines)


def main() -> None:
    py = {s.name: s for s in load_specs("tcl")}
    ru = {json.loads(l)["name"] for l in open(_REPO_ROOT / "tmp/registry-audit/tcl.rust.jsonl")}
    missing = sorted(set(py) - ru)

    def is_op(n: str) -> bool:
        return "mathop" in n or not n[:1].isalpha() or n in {"eq", "ne", "in", "ni", "max", "min"}

    named = [n for n in missing if not is_op(n)]
    mods = []
    for cmd in named:
        mod = module_name(cmd)
        (TCL_DIR / f"{mod}.rs").write_text(file_text(py[cmd]))
        mods.append(mod)

    text = MOD.read_text()
    # Insert `mod` decls after the mathop_generated decl.
    anchor = "mod mathop_generated;\n"
    decls = "".join(f"mod {m};\n" for m in sorted(mods))
    text = text.replace(anchor, anchor + decls, 1)
    # Append a third extend block with the named specs.
    calls = "\n        ".join(f"{m}::spec()," for m in sorted(mods))
    ext = "    specs.extend(mathop_generated::specs());\n    specs\n"
    new_ext = (
        "    specs.extend(mathop_generated::specs());\n"
        "    // GAP-d: simple named commands the Rust port omitted.\n"
        "    specs.extend([\n        " + calls + "\n    ]);\n"
        "    specs\n"
    )
    text = text.replace(ext, new_ext, 1)
    MOD.write_text(text)
    print(f"generated {len(mods)} named command files:", named)


if __name__ == "__main__":
    main()

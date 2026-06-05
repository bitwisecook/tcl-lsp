#!/usr/bin/env python3
"""Restore `subcommands` on iRules Rust spec files from Python.

Ports the structured subcommands (47 commands) the Rust port dropped —
each as a `SubCommand { name, arity, detail, synopsis, pure, mutator,
credential_arg, sensitive_headers }` — from the Python source of truth.
Emits a module-level `const SUBCOMMANDS` and points the spec at it.
Idempotent.

Usage: python3 scripts/registry-audit/inject_subcommands.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

from core.commands.registry.irules import irules_command_specs  # noqa: E402

RUST_DIR = _REPO_ROOT / "rust/tcl-registry/src/commands/irules"
ANY = sys.maxsize


def rust_str(s: str) -> str:
    return (
        '"'
        + (s or "").replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\t", "\\t")
        + '"'
    )


def arity_expr(ar) -> str:
    lo, hi = ar.min, ar.max
    if hi >= ANY:
        return f"Arity::at_least({lo})"
    if lo == hi:
        return f"Arity::exact({lo})"
    return f"Arity::new({lo}, {hi})"


def str_slice(items) -> str:
    items = sorted(items or ())
    if not items:
        return "&[]"
    return "&[" + ", ".join(rust_str(x) for x in items) + "]"


def subcommand_literal(sub) -> str:
    lines = [
        "    SubCommand {",
        f"        name: {rust_str(sub.name)},",
        f"        arity: {arity_expr(sub.arity)},",
        f"        detail: {rust_str(sub.detail)},",
        f"        synopsis: {rust_str(sub.synopsis)},",
    ]
    if sub.pure:
        lines.append("        pure: true,")
    if sub.mutator:
        lines.append("        mutator: true,")
    if sub.credential_arg is not None:
        lines.append(f"        credential_arg: Some({sub.credential_arg}),")
    if sub.sensitive_headers:
        lines.append(f"        sensitive_headers: {str_slice(sub.sensitive_headers)},")
    lines.append("        ..SubCommand::DEFAULT")
    lines.append("    },")
    return "\n".join(lines)


def main() -> None:
    name_re = re.compile(r'^\s*name:\s*"((?:[^"\\]|\\.)*)"', re.M)
    by_name: dict[str, Path] = {}
    for p in RUST_DIR.glob("*.rs"):
        if p.name == "mod.rs":
            continue
        mm = name_re.search(p.read_text())
        if mm:
            by_name[mm.group(1).replace('\\"', '"').replace("\\\\", "\\")] = p

    count = 0
    for spec in irules_command_specs():
        subs = spec.subcommands
        if not subs:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        if re.search(r"^\s*subcommands:", text, re.M):
            continue  # already has subcommands (idempotent)
        # Build the const, preserving Python insertion order.
        body = "\n".join(subcommand_literal(s) for s in subs.values())
        const_block = (
            "\n/// iRules subcommands ported from the Python source of truth.\n"
            "const SUBCOMMANDS: &[SubCommand] = &[\n" + body + "\n];\n"
        )
        # Insert the const after the `use crate::prelude::*;` line.
        um = re.search(r"^use crate::prelude::\*;\s*$", text, re.M)
        if not um:
            continue
        text = text[: um.end()] + "\n" + const_block + text[um.end():]
        # Point the spec at it.
        m = re.search(r"^(\s*)\.\.CommandSpec::DEFAULT", text, re.M)
        indent = m.group(1)
        text = text[: m.start()] + f"{indent}subcommands: SUBCOMMANDS,\n" + text[m.start():]
        path.write_text(text)
        count += 1
    print(f"irules: subcommands -> {count} files")


if __name__ == "__main__":
    main()

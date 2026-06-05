#!/usr/bin/env python3
"""Restore `side_effects` on iRules Rust spec files from Python.

Ports the structured side-effect hints (target / reads / writes /
connection_side) the Rust port dropped, from the Python source of truth.
Relies on the registry `SideEffectTarget` / `ConnectionSide` enums having
been expanded to the full Python set. Idempotent.

Usage: python3 scripts/registry-audit/inject_side_effects.py [group]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

GROUPS = {
    "irules": ("irules", "irules_command_specs"),
    "tcllib": ("tcllib", "tcllib_command_specs"),
    "tk": ("tk", "tk_command_specs"),
}

# Python enum-name (UPPER_SNAKE) -> Rust variant. Generic rule is
# UPPER_SNAKE -> PascalCase; `ISTATS` is the only irregular case.
_TARGET_OVERRIDE = {"ISTATS": "IStats"}
_SIDE = {
    "NONE": "None",
    "CLIENT": "Client",
    "SERVER": "Server",
    "BOTH": "Both",
    "GLOBAL": "Global",
}


def pascal(upper_snake: str) -> str:
    if upper_snake in _TARGET_OVERRIDE:
        return _TARGET_OVERRIDE[upper_snake]
    return "".join(p.capitalize() for p in upper_snake.split("_"))


def side_effect_literal(se, indent: str) -> str:
    inner = indent + "    "
    target = pascal(se.target.name)
    side = _SIDE[se.connection_side.name]
    return (
        f"{inner}SideEffect {{\n"
        f"{inner}    target: SideEffectTarget::{target},\n"
        f"{inner}    reads: {str(bool(se.reads)).lower()},\n"
        f"{inner}    writes: {str(bool(se.writes)).lower()},\n"
        f"{inner}    connection_side: ConnectionSide::{side},\n"
        f"{inner}}},"
    )


def main() -> None:
    group = sys.argv[1] if len(sys.argv) > 1 else "irules"
    mod_name, factory = GROUPS[group]
    mod = __import__(f"core.commands.registry.{mod_name}", fromlist=[factory])
    specs = getattr(mod, factory)()
    rust_dir = _REPO_ROOT / "rust/tcl-registry/src/commands" / group

    # Anchor on the command-level name (the first `name:` *after*
    # `CommandSpec {`), so a `const SUBCOMMANDS` whose subcommand
    # `name:` fields precede `spec()` doesn't shadow the command name.
    name_re = re.compile(r'CommandSpec\s*\{\s*name:\s*"((?:[^"\\]|\\.)*)"')
    by_name: dict[str, Path] = {}
    for p in rust_dir.glob("*.rs"):
        if p.name == "mod.rs":
            continue
        mm = name_re.search(p.read_text())
        if mm:
            by_name[mm.group(1).replace('\\"', '"').replace("\\\\", "\\")] = p

    count = 0
    for spec in specs:
        hints = getattr(spec, "side_effect_hints", None)
        if not hints:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        if re.search(r"^\s*side_effects:", text, re.M):
            continue
        m = re.search(r"^(\s*)\.\.CommandSpec::DEFAULT", text, re.M)
        if not m:
            continue
        indent = m.group(1)
        body = "\n".join(side_effect_literal(se, indent) for se in hints)
        block = f"side_effects: &[\n{body}\n{indent}],"
        text = text[: m.start()] + f"{indent}{block}\n" + text[m.start():]
        path.write_text(text)
        count += 1
    print(f"{group}: side_effects -> {count} files")


if __name__ == "__main__":
    main()

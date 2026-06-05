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

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import (  # noqa: E402
    files_by_name,
    has_field,
    insert_const,
    load_specs,
    rust_dir,
    set_spec_field,
)

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


def side_effect_literal(se) -> str:
    target = pascal(se.target.name)
    side = _SIDE[se.connection_side.name]
    return (
        "    SideEffect {\n"
        f"        target: SideEffectTarget::{target},\n"
        f"        reads: {str(bool(se.reads)).lower()},\n"
        f"        writes: {str(bool(se.writes)).lower()},\n"
        f"        connection_side: ConnectionSide::{side},\n"
        "    },"
    )


def main() -> None:
    group = sys.argv[1] if len(sys.argv) > 1 else "irules"
    by_name = files_by_name(rust_dir(_REPO_ROOT, group))

    count = 0
    for spec in load_specs(group):
        hints = getattr(spec, "side_effect_hints", None)
        if not hints:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        if has_field(text, "side_effects"):
            continue
        body = "\n".join(side_effect_literal(se) for se in hints)
        nt = insert_const(text, "SIDE_EFFECTS", "&[SideEffect]", body)
        if nt is None:
            continue
        text = set_spec_field(nt, "side_effects: SIDE_EFFECTS,")
        path.write_text(text)
        count += 1
    print(f"{group}: side_effects -> {count} files")


if __name__ == "__main__":
    main()

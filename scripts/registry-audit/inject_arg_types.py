#!/usr/bin/env python3
"""Inject static `arg_types` into Rust spec files from Python.

Ports the per-argument type hints (`(index, ArgTypeHint{expected,
shimmers})`) the Rust port dropped. Short inline data. Idempotent.

Usage: python3 scripts/registry-audit/inject_arg_types.py <group>
"""

from __future__ import annotations

import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import files_by_name, has_field, load_specs, rust_dir, set_spec_field  # noqa: E402

_TYPE = {
    "INT": "Int",
    "STRING": "String",
    "LIST": "List",
    "DICT": "Dict",
    "BOOLEAN": "Boolean",
    "NUMERIC": "Numeric",
    "DOUBLE": "Double",
    "CHANNEL": "Channel",
    "BYTEARRAY": "ByteArray",
    "OBJECT": "Object",
}


def hint(t) -> str:
    exp = "None"
    if t.expected is not None and t.expected.name in _TYPE:
        exp = f"Some(TclType::{_TYPE[t.expected.name]})"
    return f"ArgTypeHint {{ expected: {exp}, shimmers: {str(bool(t.shimmers)).lower()} }}"


def main() -> None:
    group = sys.argv[1] if len(sys.argv) > 1 else "tcl"
    by_name = files_by_name(rust_dir(_REPO_ROOT, group))
    count = 0
    for spec in load_specs(group):
        at = getattr(spec, "arg_types", None)
        if not at:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        if has_field(text, "arg_types"):
            continue
        body = ", ".join(f"({i}, {hint(t)})" for i, t in sorted(at.items()))
        text = set_spec_field(text, f"arg_types: &[{body}],")
        path.write_text(text)
        count += 1
    print(f"{group}: arg_types -> {count} files")


if __name__ == "__main__":
    main()

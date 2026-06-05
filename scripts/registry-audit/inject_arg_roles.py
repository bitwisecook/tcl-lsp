#!/usr/bin/env python3
"""Inject static `arg_roles` into Rust spec files from Python.

Ports the per-argument static role hints (`(index, ArgRole)`) the Rust
port dropped. Short inline data — emitted directly before
`..CommandSpec::DEFAULT`. Idempotent.

Usage: python3 scripts/registry-audit/inject_arg_roles.py <group>
"""

from __future__ import annotations

import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import files_by_name, has_field, load_specs, rust_dir, set_spec_field  # noqa: E402


def pascal(upper_snake: str) -> str:
    return "".join(p.capitalize() for p in upper_snake.split("_"))


def main() -> None:
    group = sys.argv[1] if len(sys.argv) > 1 else "tcllib"
    by_name = files_by_name(rust_dir(_REPO_ROOT, group))
    count = 0
    for spec in load_specs(group):
        roles = getattr(spec, "arg_roles", None)
        if not roles:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        if has_field(text, "arg_roles"):
            continue
        # Flatten {index: {ArgRole, ...}} -> sorted (index, role) tuples.
        tuples = sorted((idx, pascal(r.name)) for idx, rs in roles.items() for r in rs)
        body = ", ".join(f"({idx}, ArgRole::{role})" for idx, role in tuples)
        text = set_spec_field(text, f"arg_roles: &[{body}],")
        path.write_text(text)
        count += 1
    print(f"{group}: arg_roles -> {count} files")


if __name__ == "__main__":
    main()

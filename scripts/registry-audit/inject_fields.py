#!/usr/bin/env python3
"""Inject missing scalar CommandSpec fields into hand-written Rust spec files.

Surgical, idempotent porter for the rust-rewrite registry catch-up: reads
the Python source-of-truth registry for a group, and for each command
whose Rust spec file is missing a given scalar field, inserts the field
literal just before the closing ``..CommandSpec::DEFAULT``. Existing
fields are never touched (idempotent), and files that already carry the
field are skipped, so re-runs are no-ops.

Only handles flat scalar fields (the low-risk dimensions). Structured
data (hover/forms/subcommands/side_effects) is ported by dedicated
generators.

Usage: python3 scripts/registry-audit/inject_fields.py <group> <field>[,<field>...]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

# group -> (module under core.commands.registry, factory, rust subdir)
GROUPS = {
    "tcl": ("tcl", "tcl_command_specs", "tcl"),
    "stdlib": ("stdlib", "stdlib_command_specs", "stdlib"),
    "tcllib": ("tcllib", "tcllib_command_specs", "tcllib"),
    "irules": ("irules", "irules_command_specs", "irules"),
    "iapps": ("iapps", "iapps_command_specs", "iapps"),
    "tk": ("tk", "tk_command_specs", "tk"),
    "expect": ("expect", "expect_command_specs", "expect"),
}


def _rust_str(s: str) -> str:
    """Emit a Rust string literal for *s* (escaping `\\` and `"`)."""
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def _field_literal(name: str, spec: object) -> str | None:
    """Return the Rust `field: value,` line for *name*, or None to skip."""
    if name == "required_package":
        v = getattr(spec, "required_package", None)
        return f"required_package: Some({_rust_str(v)})," if v else None
    if name == "tcllib_package":
        v = getattr(spec, "tcllib_package", None)
        return f"tcllib_package: Some({_rust_str(v)})," if v else None
    if name == "warn_missing_import":
        # Default is True; only emit when the spec opts out (False) AND
        # the command is actually package-gated (the flag is meaningless
        # otherwise — e.g. internal `test*` commands carry False as a
        # base-class default with no package).
        v = getattr(spec, "warn_missing_import", True)
        gated = getattr(spec, "required_package", None) or getattr(spec, "tcllib_package", None)
        return "warn_missing_import: false," if (v is False and gated) else None
    if name == "is_namespace_exported":
        v = getattr(spec, "is_namespace_exported", False)
        return "is_namespace_exported: true," if v else None
    if name == "deprecated_replacement":
        v = getattr(spec, "deprecated_replacement_name", None)
        return f"deprecated_replacement: Some({_rust_str(v)})," if v else None
    if name == "xc_translatable":
        v = getattr(spec, "xc_translatable", None)
        if v is None:
            return None
        return f"xc_translatable: Some({str(bool(v)).lower()}),"
    raise SystemExit(f"unsupported field: {name}")


def load_specs(group: str):
    mod_name, factory, _ = GROUPS[group]
    mod = __import__(
        f"core.commands.registry.{mod_name}", fromlist=[factory]
    )
    return getattr(mod, factory)()


sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import files_by_name as rust_files_by_name  # noqa: E402


def inject(path: Path, field_lines: list[str]) -> bool:
    text = path.read_text()
    # Skip any field already present.
    pending = [
        ln for ln in field_lines
        if re.search(r"^\s*" + re.escape(ln.split(":", 1)[0]) + r"\s*:", text, re.M) is None
    ]
    if not pending:
        return False
    # Insert before the `..CommandSpec::DEFAULT` line, matching its indent.
    m = re.search(r"^(\s*)\.\.CommandSpec::DEFAULT", text, re.M)
    if not m:
        return False
    indent = m.group(1)
    block = "".join(f"{indent}{ln}\n" for ln in pending)
    new = text[: m.start()] + block + text[m.start():]
    path.write_text(new)
    return True


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    group, fields = sys.argv[1], sys.argv[2].split(",")
    rust_dir = _REPO_ROOT / "rust/tcl-registry/src/commands" / GROUPS[group][2]
    by_name = rust_files_by_name(rust_dir)
    specs = load_specs(group)
    changed = 0
    missing_file = 0
    for spec in specs:
        lines = [ln for f in fields if (ln := _field_literal(f, spec))]
        if not lines:
            continue
        path = by_name.get(spec.name)
        if path is None:
            missing_file += 1
            continue
        if inject(path, lines):
            changed += 1
    print(f"{group}: injected into {changed} files ({missing_file} commands had no rust file)")


if __name__ == "__main__":
    main()

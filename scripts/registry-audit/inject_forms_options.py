#!/usr/bin/env python3
"""Restore `forms` + `options` on iRules Rust spec files from Python.

The Rust port dropped the per-command invocation forms (999) and most
options (54→5). This injects:

  * `forms:` — one `FormSpec { kind, synopsis }` per Python form.
  * `options:` — the union of every form's options as
    `OptionSpec { name, takes_value, value_hint, detail, dialects }`.

Both are sourced from the Python registry (reference standard) and
inserted before `..CommandSpec::DEFAULT`. Idempotent: a file that
already carries the field is left untouched.

Usage: python3 scripts/registry-audit/inject_forms_options.py [group]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

GROUPS = {"irules": ("irules", "irules_command_specs")}

_FORMKIND = {"DEFAULT": "Default", "GETTER": "Getter", "SETTER": "Setter"}


def rust_str(s: str) -> str:
    return (
        '"'
        + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\t", "\\t")
        + '"'
    )


def form_literal(form, indent: str) -> str:
    kind = _FORMKIND.get(form.kind.name, "Default")
    return (
        f"{indent}    FormSpec {{ kind: FormKind::{kind}, "
        f"synopsis: {rust_str(form.synopsis or '')} }},"
    )


def option_literal(opt, indent: str) -> str:
    detail = getattr(opt, "detail", "") or ""
    return (
        f"{indent}    OptionSpec {{ name: {rust_str(opt.name)}, "
        f"takes_value: {str(bool(opt.takes_value)).lower()}, "
        f"value_hint: {rust_str(getattr(opt, 'value_hint', '') or '')}, "
        f"detail: {rust_str(detail)}, dialects: None }},"
    )


def collect_options(forms):
    """Union of all form options, de-duplicated by name (first wins)."""
    seen: set[str] = set()
    out = []
    for f in forms:
        for o in getattr(f, "options", ()) or ():
            if o.name not in seen:
                seen.add(o.name)
                out.append(o)
    return out


def main() -> None:
    group = sys.argv[1] if len(sys.argv) > 1 else "irules"
    mod_name, factory = GROUPS[group]
    mod = __import__(f"core.commands.registry.{mod_name}", fromlist=[factory])
    specs = getattr(mod, factory)()
    rust_dir = _REPO_ROOT / "rust/tcl-registry/src/commands" / group

    name_re = re.compile(r'^\s*name:\s*"((?:[^"\\]|\\.)*)"', re.M)
    by_name: dict[str, Path] = {}
    for p in rust_dir.glob("*.rs"):
        if p.name == "mod.rs":
            continue
        mm = name_re.search(p.read_text())
        if mm:
            by_name[mm.group(1).replace('\\"', '"').replace("\\\\", "\\")] = p

    f_count = o_count = 0
    for spec in specs:
        forms = spec.forms or ()
        if not forms:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        m = re.search(r"^(\s*)\.\.CommandSpec::DEFAULT", text, re.M)
        if not m:
            continue
        indent = m.group(1)
        has_field = lambda name: re.search(r"^\s*" + name + r":", text, re.M) is not None
        blocks: list[str] = []
        if not has_field("forms"):
            body = "\n".join(form_literal(f, indent) for f in forms)
            blocks.append(f"forms: &[\n{body}\n{indent}],")
            f_count += 1
        opts = collect_options(forms)
        if opts and not has_field("options"):
            body = "\n".join(option_literal(o, indent) for o in opts)
            blocks.append(f"options: &[\n{body}\n{indent}],")
            o_count += 1
        if not blocks:
            continue
        inject = "".join(f"{indent}{b}\n" for b in blocks)
        text = text[: m.start()] + inject + text[m.start():]
        path.write_text(text)
    print(f"{group}: forms -> {f_count} files, options -> {o_count} files")


if __name__ == "__main__":
    main()

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

_FORMKIND = {"DEFAULT": "Default", "GETTER": "Getter", "SETTER": "Setter"}


def rust_str(s: str) -> str:
    return (
        '"'
        + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\t", "\\t")
        + '"'
    )


def form_literal(form) -> str:
    kind = _FORMKIND.get(form.kind.name, "Default")
    return f"    FormSpec {{ kind: FormKind::{kind}, synopsis: {rust_str(form.synopsis or '')} }},"


def option_literal(opt) -> str:
    detail = getattr(opt, "detail", "") or ""
    return (
        f"    OptionSpec {{ name: {rust_str(opt.name)}, "
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
    by_name = files_by_name(rust_dir(_REPO_ROOT, group))

    f_count = o_count = 0
    for spec in load_specs(group):
        forms = spec.forms or ()
        if not forms:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        if not has_field(text, "forms"):
            body = "\n".join(form_literal(f) for f in forms)
            nt = insert_const(text, "FORMS", "&[FormSpec]", body)
            if nt:
                text = set_spec_field(nt, "forms: FORMS,")
                f_count += 1
        opts = collect_options(forms)
        if opts and not has_field(text, "options"):
            body = "\n".join(option_literal(o) for o in opts)
            nt = insert_const(text, "OPTIONS", "&[OptionSpec]", body)
            if nt:
                text = set_spec_field(nt, "options: OPTIONS,")
                o_count += 1
        path.write_text(text)
    print(f"{group}: forms -> {f_count} files, options -> {o_count} files")


if __name__ == "__main__":
    main()

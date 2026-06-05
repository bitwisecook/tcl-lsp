#!/usr/bin/env python3
"""Restore full `hover` data on iRules Rust spec files from Python.

The Rust port reduced every iRules hover to `HoverSnippet::brief(summary,
synopsis, "F5 iRules")`, dropping the real doc-source URL, examples,
return-value, and extended snippet, and truncating 124 summaries /
diverging 143 synopses. This regenerator replaces the whole `hover:
Some(...)` value with a full `HoverSnippet { .. }` literal sourced from
the Python registry (the reference standard). Idempotent.

Usage: python3 scripts/registry-audit/inject_hover.py [group]   (default: irules)
"""

from __future__ import annotations

import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

import re  # noqa: E402

GROUPS = {
    "irules": ("irules", "irules_command_specs"),
    "tcllib": ("tcllib", "tcllib_command_specs"),
}


def rust_str(s: str) -> str:
    s = (
        s.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )
    return '"' + s + '"'


def str_slice(items) -> str:
    if not items:
        return "&[]"
    return "&[" + ", ".join(rust_str(x) for x in items) + "]"


def hover_literal(h, indent: str) -> str:
    inner = indent + "    "
    return (
        "hover: Some(HoverSnippet {\n"
        f"{inner}summary: {rust_str(h.summary or '')},\n"
        f"{inner}synopsis: {str_slice(list(h.synopsis or ()))},\n"
        f"{inner}snippet: {rust_str(h.snippet or '')},\n"
        f"{inner}source: {rust_str(h.source or '')},\n"
        f"{inner}examples: {rust_str(h.examples or '')},\n"
        f"{inner}return_value: {rust_str(h.return_value or '')},\n"
        f"{indent}}}),"
    )


def find_hover_span(text: str) -> tuple[int, int, str] | None:
    """Return (start, end, indent) of the `hover: Some(...)` value + trailing comma.

    Balances parens/braces while skipping Rust string literals so a `(`
    or `}` inside a synopsis/summary string doesn't desync the scan.
    """
    m = re.search(r"^(?P<indent>[ \t]*)hover:\s*Some\(", text, re.M)
    if not m:
        return None
    indent = m.group("indent")
    i = m.end()  # just after the `Some(`
    depth = 1
    in_str = False
    esc = False
    while i < len(text):
        c = text[i]
        if in_str:
            if esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == '"':
                in_str = False
        else:
            if c == '"':
                in_str = True
            elif c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
                if depth == 0:
                    # `i` is the `)` closing `Some(`. Consume a trailing comma.
                    end = i + 1
                    if end < len(text) and text[end] == ",":
                        end += 1
                    return m.start(), end, indent
        i += 1
    return None


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

    changed = 0
    for spec in specs:
        h = getattr(spec, "hover", None)
        if h is None:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        span = find_hover_span(text)
        if span is None:
            continue
        start, end, indent = span
        new_hover = hover_literal(h, indent)
        new_text = text[:start] + new_hover + text[end:]
        if new_text != text:
            path.write_text(new_text)
            changed += 1
    print(f"{group}: hover restored in {changed} files")


if __name__ == "__main__":
    main()

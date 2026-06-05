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

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import files_by_name, load_specs, rust_dir  # noqa: E402


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


def hover_literal(h, indent: str, extra_synopsis: list[str] | None = None) -> str:
    inner = indent + "    "
    # Synopsis is the *union* of Python's and any Rust-only lines, so the
    # regen never reduces a Rust hover that carried more synopsis lines.
    syn = list(h.synopsis or ())
    for s in extra_synopsis or []:
        if s not in syn:
            syn.append(s)
    return (
        "hover: Some(HoverSnippet {\n"
        f"{inner}summary: {rust_str(h.summary or '')},\n"
        f"{inner}synopsis: {str_slice(syn)},\n"
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
    specs = load_specs(group)
    by_name = files_by_name(rust_dir(_REPO_ROOT, group))

    # Current Rust synopsis per command (from the latest dump), so the
    # union-merge can preserve Rust-only synopsis lines.
    rust_syn: dict[str, list[str]] = {}
    dump = _REPO_ROOT / "tmp/registry-audit" / f"{group}.rust.jsonl"
    if dump.exists():
        import json
        for line in dump.read_text().splitlines():
            d = json.loads(line)
            rust_syn[d["name"]] = d.get("synopsis") or []

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
        new_hover = hover_literal(h, indent, rust_syn.get(spec.name))
        new_text = text[:start] + new_hover + text[end:]
        if new_text != text:
            path.write_text(new_text)
            changed += 1
    print(f"{group}: hover restored in {changed} files")


if __name__ == "__main__":
    main()

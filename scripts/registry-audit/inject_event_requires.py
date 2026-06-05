#!/usr/bin/env python3
"""Inject `event_requires` + `excluded_events` into iRules Rust spec files.

Ports the iRules layer-based event requirements (transport / profiles /
also_in / side / init-only / flow / capability) and the excluded-event
lists from the Python source of truth into the hand-written Rust spec
files. Idempotent: files that already carry the field are skipped.

Usage: python3 scripts/registry-audit/inject_event_requires.py
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


def rust_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def str_slice(items) -> str:
    items = sorted(items or ())
    if not items:
        return "&[]"
    return "&[" + ", ".join(rust_str(x) for x in items) + "]"


def is_meaningful(er) -> bool:
    """True when *er* imposes any real requirement.

    Python sets a non-None but all-default `EventRequires` on ~563
    iRules commands; that is semantically identical to `None` (no
    requirement) and audit-equivalent, so we leave those as the Rust
    `None` default rather than emitting noise structs.
    """
    return er is not None and any(
        [
            er.client_side,
            er.server_side,
            er.transport is not None,
            bool(er.profiles),
            bool(er.also_in),
            er.init_only,
            er.flow,
            er.capability is not None,
        ]
    )


def event_requires_literal(er, indent: str) -> str:
    inner = indent + "    "
    transport = f"Some({rust_str(er.transport)})" if er.transport else "None"
    capability = f"Some({rust_str(er.capability)})" if er.capability else "None"
    return (
        "event_requires: Some(EventRequires {\n"
        f"{inner}client_side: {str(er.client_side).lower()},\n"
        f"{inner}server_side: {str(er.server_side).lower()},\n"
        f"{inner}transport: {transport},\n"
        f"{inner}profiles: {str_slice(er.profiles)},\n"
        f"{inner}also_in: {str_slice(er.also_in)},\n"
        f"{inner}init_only: {str(er.init_only).lower()},\n"
        f"{inner}flow: {str(er.flow).lower()},\n"
        f"{inner}capability: {capability},\n"
        f"{indent}}}),"
    )


def rust_files_by_name() -> dict[str, Path]:
    out: dict[str, Path] = {}
    name_re = re.compile(r'^\s*name:\s*"((?:[^"\\]|\\.)*)"', re.M)
    for p in RUST_DIR.glob("*.rs"):
        if p.name == "mod.rs":
            continue
        m = name_re.search(p.read_text())
        if m:
            out[m.group(1).replace('\\"', '"').replace("\\\\", "\\")] = p
    return out


def main() -> None:
    by_name = rust_files_by_name()
    er_count = ex_count = 0
    for spec in irules_command_specs():
        er = getattr(spec, "event_requires", None)
        if not is_meaningful(er):
            er = None
        excluded = getattr(spec, "excluded_events", ())
        if not er and not excluded:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        m = re.search(r"^(\s*)\.\.CommandSpec::DEFAULT", text, re.M)
        if not m:
            continue
        indent = m.group(1)

        def has_field(name: str, text: str = text) -> bool:
            return re.search(r"^\s*" + name + r":", text, re.M) is not None

        blocks: list[str] = []
        if er and not has_field("event_requires"):
            blocks.append(event_requires_literal(er, indent))
            er_count += 1
        if excluded and not has_field("excluded_events"):
            blocks.append(f"excluded_events: {str_slice(excluded)},")
            ex_count += 1
        if not blocks:
            continue
        inject = "".join(f"{indent}{b}\n" for b in blocks)
        text = text[: m.start()] + inject + text[m.start() :]
        path.write_text(text)
    print(f"irules: event_requires -> {er_count} files, excluded_events -> {ex_count} files")


if __name__ == "__main__":
    main()

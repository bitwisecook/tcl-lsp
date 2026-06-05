#!/usr/bin/env python3
"""Enrich hand-written tcl spec files so each carries at least as much as
its Python equivalent: append the missing subcommands / options to the
existing arrays, and replace the inline arg_roles / arg_types / add
return_type where Rust has fewer than Python.

Merges (never reduces) — existing richer Rust per-subcommand data is
kept; only the names Python has and Rust lacks are appended.

Run after run_all.sh (uses the fresh dump to compute deficiencies).
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))
sys.path.insert(0, str(Path(__file__).resolve().parent))

from _groups import files_by_name, load_specs, rust_dir, set_spec_field  # noqa: E402
from inject_subcommands import subcommand_literal  # noqa: E402
from inject_forms_options import option_literal, collect_options  # noqa: E402
from inject_arg_roles import pascal  # noqa: E402
from inject_arg_types import hint as argtype_hint  # noqa: E402

TCL = rust_dir(_REPO_ROOT, "tcl")
DUMP = {json.loads(l)["name"]: json.loads(l) for l in open(_REPO_ROOT / "tmp/registry-audit/tcl.rust.jsonl")}


def array_close(text: str, open_idx: int) -> int:
    """Return the index of the `]` that closes the `&[` at/after open_idx,
    skipping Rust string literals."""
    i = text.index("[", open_idx) + 1
    depth = 1
    in_str = esc = False
    while i < len(text):
        c = text[i]
        if in_str:
            if esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "[":
            depth += 1
        elif c == "]":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError("unbalanced array")


def find_field_array(text: str, field: str) -> tuple[int, int] | None:
    """Locate the `&[ ... ]` slice for `field:` — following a `const`/
    `static` alias if the field is set to one. Returns (insert_before_idx,
    close_idx) for appending before the closing `]`."""
    m = re.search(rf"^\s*{field}:\s*(&\[|([A-Z_]+),)", text, re.M)
    if not m:
        return None
    if m.group(1).startswith("&["):
        opn = m.start(1)
    else:
        const = m.group(2)
        cm = re.search(rf"(?:const|static)\s+{const}\s*:[^=]*=\s*&\[", text)
        if not cm:
            return None
        opn = cm.end() - 2
    close = array_close(text, opn)
    return opn, close


def append_items(text: str, field: str, literals: list[str]) -> str:
    loc = find_field_array(text, field)
    if loc is None:
        return text
    _, close = loc
    block = "\n".join(literals)
    return text[:close] + block + "\n" + text[close:]


def main() -> None:
    by_name = files_by_name(TCL)
    py = {s.name: s for s in load_specs("tcl")}

    # 1. subcommands — append missing.
    for cmd in ["chan", "dict", "encoding", "interp", "trace"]:
        spec, path = py[cmd], by_name[cmd]
        have = set(DUMP[cmd]["subcommands"])
        missing = [s for n, s in spec.subcommands.items() if n not in have]
        if missing:
            lits = [subcommand_literal(s) for s in missing]
            path.write_text(append_items(path.read_text(), "subcommands", lits))
            print(f"{cmd}: +{len(missing)} subcommands")

    # 2. options — append missing.
    for cmd in ["chan", "clock"]:
        spec, path = py[cmd], by_name[cmd]
        have = set(DUMP[cmd]["options"])
        missing = [o for o in collect_options(spec.forms or ()) if o.name not in have]
        if missing:
            lits = [option_literal(o) for o in missing]
            path.write_text(append_items(path.read_text(), "options", lits))
            print(f"{cmd}: +{len(missing)} options")

    # 3. arg_roles / arg_types — replace the inline field with the full set.
    for cmd in ["lassign", "scan", "lindex", "lrange"]:
        spec, path = py[cmd], by_name[cmd]
        text = path.read_text()
        roles = spec.arg_roles or {}
        if roles:
            tuples = sorted((i, pascal(r.name)) for i, rs in roles.items() for r in rs)
            body = ", ".join(f"({i}, ArgRole::{r})" for i, r in tuples)
            text = re.sub(r"^\s*arg_roles:\s*&\[[^\]]*\],\n", "", text, flags=re.M)
            text = set_spec_field(text, f"arg_roles: &[{body}],")
        at = spec.arg_types or {}
        if at:
            body = ", ".join(f"({i}, {argtype_hint(t)})" for i, t in sorted(at.items()))
            text = re.sub(r"^\s*arg_types:\s*&\[[^\]]*\],\n", "", text, flags=re.M)
            text = set_spec_field(text, f"arg_types: &[{body}],")
        path.write_text(text)
        print(f"{cmd}: arg_roles/arg_types refreshed")

    # 4. return_type — add where Python has one and Rust lacks it.
    _RT = {"INT": "Int", "STRING": "String", "LIST": "List", "BOOLEAN": "Boolean",
           "NUMERIC": "Numeric", "DOUBLE": "Double", "DICT": "Dict"}
    for cmd in ["scan"]:
        spec, path = py[cmd], by_name[cmd]
        rt = spec.return_type
        text = path.read_text()
        if rt and rt.name in _RT and not re.search(r"^\s*return_type:", text, re.M):
            text = set_spec_field(text, f"return_type: Some(TclType::{_RT[rt.name]}),")
            path.write_text(text)
            print(f"{cmd}: +return_type")


if __name__ == "__main__":
    main()

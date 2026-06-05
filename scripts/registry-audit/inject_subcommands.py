#!/usr/bin/env python3
"""Restore `subcommands` (full content) on generated Rust spec files from
Python — name / arity / detail / synopsis / pure / mutator / destructive /
returns_path / is_unescape / return_type / options / arg_values /
side_effects / safe_on_uninit / credential_arg / sensitive_headers /
taint_transform / inferred_storage_type / cfg_rewrite_name.

Emits a module-level `const SUBCOMMANDS` and points the spec at it; if a
generator-owned `const SUBCOMMANDS` already exists it is regenerated.
Hand-written subcommand tables (no generator marker) are left untouched.

Usage: python3 scripts/registry-audit/inject_subcommands.py <group>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import load_specs, rust_dir  # noqa: E402

ANY = sys.maxsize
MARKER = "/// Subcommands ported from the Python source of truth."
_TYPE = {"INT": "Int", "STRING": "String", "LIST": "List", "DICT": "Dict",
         "BOOLEAN": "Boolean", "NUMERIC": "Numeric", "DOUBLE": "Double", "CHANNEL": "Channel"}
_STORAGE = {"DICT": "Dict", "LIST": "List", "ARRAY": "Array"}
_TARGET = {}  # filled lazily via pascal
_SIDE = {"NONE": "None", "CLIENT": "Client", "SERVER": "Server", "BOTH": "Both", "GLOBAL": "Global"}


def rust_str(s: str) -> str:
    return ('"' + (s or "").replace("\\", "\\\\").replace('"', '\\"')
            .replace("\n", "\\n").replace("\t", "\\t") + '"')


def arity_expr(ar) -> str:
    lo, hi = ar.min, ar.max
    if hi >= ANY:
        return f"Arity::at_least({lo})"
    return f"Arity::exact({lo})" if lo == hi else f"Arity::new({lo}, {hi})"


def str_slice(items) -> str:
    items = sorted(items or ())
    return "&[]" if not items else "&[" + ", ".join(rust_str(x) for x in items) + "]"


def pascal(s: str) -> str:
    return {"ISTATS": "IStats"}.get(s, "".join(p.capitalize() for p in s.split("_")))


def colour(v) -> str | None:
    if v is None:
        return None
    names = sorted(m.name for m in type(v) if (v & m) == m and m.value)
    return " | ".join(f"TaintColour::{n}" for n in names)


def side_effect_lit(se, ind: str) -> str:
    return (f"{ind}SideEffect {{ target: SideEffectTarget::{pascal(se.target.name)}, "
            f"reads: {str(bool(se.reads)).lower()}, writes: {str(bool(se.writes)).lower()}, "
            f"connection_side: ConnectionSide::{_SIDE[se.connection_side.name]} }},")


def option_lit(o, ind: str) -> str:
    return (f"{ind}OptionSpec {{ name: {rust_str(o.name)}, takes_value: {str(bool(o.takes_value)).lower()}, "
            f"value_hint: {rust_str(getattr(o, 'value_hint', '') or '')}, "
            f"detail: {rust_str(getattr(o, 'detail', '') or '')}, dialects: None }},")


def arg_values_lit(av: dict, ind: str) -> str:
    inner = ind + "    "
    groups = []
    for idx, vals in sorted(av.items()):
        items = ", ".join(
            f"ArgValue {{ value: {rust_str(v.value)}, detail: {rust_str(getattr(v, 'detail', '') or '')} }}"
            for v in vals
        )
        groups.append(f"{inner}({idx}, &[{items}]),")
    return "&[\n" + "\n".join(groups) + f"\n{ind}]"


def subcommand_literal(sub) -> str:
    L = ["    SubCommand {",
         f"        name: {rust_str(sub.name)},",
         f"        arity: {arity_expr(sub.arity)},",
         f"        detail: {rust_str(sub.detail)},",
         f"        synopsis: {rust_str(sub.synopsis)},"]
    rt = sub.return_type
    if rt is not None and rt.name in _TYPE:
        L.append(f"        return_type: Some(TclType::{_TYPE[rt.name]}),")
    if sub.pure:
        L.append("        pure: true,")
    if sub.mutator:
        L.append("        mutator: true,")
    if getattr(sub, "destructive", False):
        L.append("        destructive: true,")
    if getattr(sub, "returns_path", False):
        L.append("        returns_path: true,")
    if getattr(sub, "is_unescape_command", False):
        L.append("        is_unescape: true,")
    if getattr(sub, "loop_list_header", False):
        L.append("        loop_list_header: true,")
    if getattr(sub, "creates_scope_alias", False):
        L.append("        creates_scope_alias: true,")
    if sub.safe_on_uninit is not None:
        L.append("        safe_on_uninit: Some(DialectSet::empty()),")
    ist = getattr(sub, "inferred_storage_type", None)
    if ist is not None and ist.name in _STORAGE:
        L.append(f"        inferred_storage_type: Some(StorageType::{_STORAGE[ist.name]}),")
    if getattr(sub, "cfg_rewrite_name", None):
        L.append(f"        cfg_rewrite_name: Some({rust_str(sub.cfg_rewrite_name)}),")
    if sub.credential_arg is not None:
        L.append(f"        credential_arg: Some({sub.credential_arg}),")
    if sub.sensitive_headers:
        L.append(f"        sensitive_headers: {str_slice(sub.sensitive_headers)},")
    tt = colour(getattr(sub, "taint_transform", None))
    if tt:
        L.append(f"        taint_transform: Some({tt}),")
    if getattr(sub, "taint_output_sink", None):
        L.append(f"        taint_output_sink: Some({rust_str(sub.taint_output_sink)}),")
    if sub.options:
        body = "\n".join(option_lit(o, "            ") for o in sub.options)
        L.append(f"        options: &[\n{body}\n        ],")
    if getattr(sub, "arg_values", None):
        L.append(f"        arg_values: {arg_values_lit(sub.arg_values, '        ')},")
    hints = getattr(sub, "side_effect_hints", None)
    if hints:
        body = "\n".join(side_effect_lit(se, "            ") for se in hints)
        L.append(f"        side_effects: &[\n{body}\n        ],")
    L.append("        ..SubCommand::DEFAULT")
    L.append("    },")
    return "\n".join(L)


def main() -> None:
    group = sys.argv[1] if len(sys.argv) > 1 else "irules"
    name_re = re.compile(r'CommandSpec\s*\{\s*name:\s*"((?:[^"\\]|\\.)*)"')
    by_name: dict[str, Path] = {}
    for p in rust_dir(_REPO_ROOT, group).glob("*.rs"):
        if p.name == "mod.rs":
            continue
        mm = name_re.search(p.read_text())
        if mm:
            by_name[mm.group(1).replace('\\"', '"').replace("\\\\", "\\")] = p

    count = 0
    for spec in load_specs(group):
        subs = spec.subcommands
        if not subs:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        body = "\n".join(subcommand_literal(s) for s in subs.values())
        const_block = f"\n{MARKER}\nconst SUBCOMMANDS: &[SubCommand] = &[\n{body}\n];\n"
        marker_re = r"/// (?:iRules s|S)ubcommands ported from the Python source of truth\."
        if re.search(marker_re, text):
            # Regenerate the generator-owned const in place.
            text = re.sub(
                marker_re + r"\nconst SUBCOMMANDS: &\[SubCommand\] = &\[.*?\n\];\n",
                const_block.lstrip("\n"),
                text,
                count=1,
                flags=re.S,
            )
        elif not re.search(r"^\s*subcommands:", text, re.M):
            um = re.search(r"^use crate::prelude::\*;\s*$", text, re.M)
            if not um:
                continue
            text = text[: um.end()] + "\n" + const_block + text[um.end():]
            m = re.search(r"^(\s*)\.\.CommandSpec::DEFAULT", text, re.M)
            text = text[: m.start()] + f"{m.group(1)}subcommands: SUBCOMMANDS,\n" + text[m.start():]
        else:
            continue  # hand-written subcommands — leave untouched
        path.write_text(text)
        count += 1
    print(f"{group}: subcommands -> {count} files")


if __name__ == "__main__":
    main()

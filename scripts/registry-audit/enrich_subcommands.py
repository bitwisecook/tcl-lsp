#!/usr/bin/env python3
"""Enrich hand-written subcommand literals so each carries at least as
much as its Python equivalent. Block-local edits matched by subcommand
name: inserts missing scalar/bool/list fields and replaces a
return_type / arg_roles / arg_types that is present but thinner than
Python. Preserves everything else the Rust block declares (hooks,
const-fold, hover, taint, …). Idempotent.

Usage: python3 scripts/registry-audit/enrich_subcommands.py <group>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from _groups import files_by_name, load_specs, rust_dir  # noqa: E402
from inject_arg_roles import pascal as role_pascal  # noqa: E402
from inject_arg_types import hint as argtype_hint  # noqa: E402
from inject_subcommands import arg_values_lit, option_lit, rust_str, side_effect_lit  # noqa: E402

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
_FORMKIND = {"DEFAULT": "Default", "GETTER": "Getter", "SETTER": "Setter"}
_BOOLS = [
    "pure",
    "mutator",
    "destructive",
    "returns_path",
    "loop_list_header",
    "creates_scope_alias",
]


def block_spans(text: str):
    """Yield (start, end) of each top-level `SubCommand { .. }` literal."""
    for m in re.finditer(r"\bSubCommand\s*\{", text):
        i, depth, in_str, esc = m.end(), 1, False, False
        while i < len(text) and depth:
            c = text[i]
            if in_str:
                esc = (c == "\\") and not esc
                if c == '"' and not esc:
                    in_str = False
            elif c == '"':
                in_str = True
            elif c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            i += 1
        yield m.start(), i


def n_tuples(block: str, field: str) -> int:
    m = re.search(rf"{field}:\s*&\[(.*?)\],", block, re.S)
    return 0 if not m else len(re.findall(r"\(\d+,", m.group(1)))


def enrich_block(block: str, sub) -> str:
    """Return *block* augmented to carry ≥ Python's data."""
    indent = "        "  # field indent inside a SubCommand literal
    dflt = "..SubCommand::DEFAULT"
    if dflt not in block:
        return block
    adds: list[str] = []

    # return_type — insert or replace when thinner/wrong.
    rt = sub.return_type
    if rt is not None and rt.name in _TYPE:
        want = f"return_type: Some(TclType::{_TYPE[rt.name]}),"
        if "return_type:" not in block:
            adds.append(indent + want)
        elif f"TclType::{_TYPE[rt.name]}" not in block:
            block = re.sub(r"return_type:\s*[^,]+,", want, block, count=1)

    for f in _BOOLS:
        if getattr(sub, f, False) and re.search(rf"^\s*{f}:", block, re.M) is None:
            adds.append(f"{indent}{f}: true,")
    if getattr(sub, "is_unescape_command", False) and "is_unescape:" not in block:
        adds.append(f"{indent}is_unescape: true,")
    if sub.safe_on_uninit is not None and "safe_on_uninit:" not in block:
        adds.append(f"{indent}safe_on_uninit: Some(DialectSet::empty()),")
    if getattr(sub, "cfg_rewrite_name", None) and "cfg_rewrite_name:" not in block:
        adds.append(f"{indent}cfg_rewrite_name: Some({rust_str(sub.cfg_rewrite_name)}),")

    # arg_roles / arg_types — replace when Python carries more entries.
    roles = sub.arg_roles or {}
    if roles:
        tuples = sorted((i, role_pascal(r.name)) for i, rs in roles.items() for r in rs)
        body = ", ".join(f"({i}, ArgRole::{r})" for i, r in tuples)
        if "arg_roles:" not in block:
            adds.append(f"{indent}arg_roles: &[{body}],")
        elif n_tuples(block, "arg_roles") < len(tuples):
            block = re.sub(
                r"arg_roles:\s*&\[.*?\],", f"arg_roles: &[{body}],", block, count=1, flags=re.S
            )
    at = sub.arg_types or {}
    if at:
        body = ", ".join(f"({i}, {argtype_hint(t)})" for i, t in sorted(at.items()))
        if "arg_types:" not in block:
            adds.append(f"{indent}arg_types: &[{body}],")
        elif n_tuples(block, "arg_types") < len(at):
            block = re.sub(
                r"arg_types:\s*&\[.*?\],", f"arg_types: &[{body}],", block, count=1, flags=re.S
            )

    if sub.options and "options:" not in block:
        body = "\n".join(option_lit(o, indent + "    ") for o in sub.options)
        adds.append(f"{indent}options: &[\n{body}\n{indent}],")
    if getattr(sub, "arg_values", None) and "arg_values:" not in block:
        adds.append(f"{indent}arg_values: {arg_values_lit(sub.arg_values, indent)},")
    hints = getattr(sub, "side_effect_hints", None)
    if hints and "side_effects:" not in block:
        body = "\n".join(side_effect_lit(se, indent + "    ") for se in hints)
        adds.append(f"{indent}side_effects: &[\n{body}\n{indent}],")
    forms = getattr(sub, "forms", ()) or ()
    if forms and "subcommand_forms:" not in block:
        items = ", ".join(
            f"SubCommandForm {{ name: {rust_str(_FORMKIND.get(f.kind.name, 'Default').lower())}, "
            f"..SubCommandForm::DEFAULT }}"
            for f in forms
        )
        adds.append(f"{indent}subcommand_forms: &[{items}],")

    if adds:
        pos = block.rfind(dflt)
        block = block[:pos] + "".join(a + "\n" for a in adds) + block[pos:]
    return block


def main() -> None:
    group = sys.argv[1] if len(sys.argv) > 1 else "tcl"
    by_name = files_by_name(rust_dir(_REPO_ROOT, group))
    changed = 0
    for spec in load_specs(group):
        subs = spec.subcommands
        if not subs:
            continue
        path = by_name.get(spec.name)
        if path is None:
            continue
        text = path.read_text()
        spans = list(block_spans(text))
        new = text
        for start, end in reversed(spans):
            block = text[start:end]
            nm = re.search(r'name:\s*"((?:[^"\\]|\\.)*)"', block)
            if not nm:
                continue
            sub = subs.get(nm.group(1))
            if sub is None:
                continue
            nb = enrich_block(block, sub)
            if nb != block:
                new = new[:start] + nb + new[end:]
        if new != text:
            path.write_text(new)
            changed += 1
    print(f"{group}: subcommands enriched in {changed} files")


if __name__ == "__main__":
    main()

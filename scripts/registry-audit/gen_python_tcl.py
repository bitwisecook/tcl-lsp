#!/usr/bin/env python3
"""Add the Tcl 8.7/9.0 commands the Rust port has but Python lacks (item f).

Generates minimal Python `CommandDef` files from the Rust spec data
(name / dialects / arity / hover / return_type) and wires them into
`core/commands/registry/tcl/__init__.py`'s import list. Source of truth
for these newer commands is the Rust port; the Python registry was
behind.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
PY_DIR = _REPO_ROOT / "core/commands/registry/tcl"
INIT = PY_DIR / "__init__.py"

# command -> (python module name, rust dump source)
WANT = {
    "const": "const",
    "lpop": "lpop",
    "coroinject": "coroinject",
    "coroprobe": "coroprobe",
    "timerate": "timerate",
    "readFile": "readfile",
    "writeFile": "writefile",
    "foreachLine": "foreachline",
    "tcl::idna": "tcl_idna",
    "tcl::process": "tcl_process",
}

_TYPE = {"String": "STRING", "Int": "INT", "List": "LIST", "Boolean": "BOOLEAN"}


def py_str(s: str) -> str:
    return repr(s)


def class_name(cmd: str) -> str:
    base = cmd.replace("::", " ").replace("_", " ")
    return "".join(p.capitalize() for p in base.split()) + "Command"


def file_text(d: dict) -> str:
    name = d["name"]
    if d["dialects_all"]:
        dialects = "None"
    else:
        dialects = "frozenset({" + ", ".join(py_str(t) for t in sorted(d["dialects"])) + "})"
    amin, amax = d["arity_min"], d["arity_max"]
    arity = f"Arity({amin})" if amax is None else f"Arity({amin}, {amax})"
    syn = (
        "("
        + ", ".join(py_str(s) for s in d["synopsis"])
        + ("," if len(d["synopsis"]) == 1 else "")
        + ")"
    )
    rt = d.get("return_type")
    rt_line = f"            return_type=TclType.{_TYPE[rt]},\n" if rt in _TYPE else ""
    return (
        f'"""{name} -- {d["summary"]}"""\n\n'
        "from __future__ import annotations\n\n"
        "from ....compiler.types import TclType\n"
        "from .._base import CommandDef\n"
        "from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec\n"
        "from ..signatures import Arity\n"
        "from ._base import register\n\n\n"
        "@register\n"
        f"class {class_name(name)}(CommandDef):\n"
        f"    name = {py_str(name)}\n\n"
        "    @classmethod\n"
        "    def spec(cls) -> CommandSpec:\n"
        "        return CommandSpec(\n"
        f"            name={py_str(name)},\n"
        f"            dialects={dialects},\n"
        "            hover=HoverSnippet(\n"
        f"                summary={py_str(d['summary'])},\n"
        f"                synopsis={syn},\n"
        f"                source={py_str(d['source'])},\n"
        "            ),\n"
        "            forms=(\n"
        f"                FormSpec(kind=FormKind.DEFAULT, synopsis={py_str(d['synopsis'][0])}),\n"
        "            ),\n"
        f"            validation=ValidationSpec(arity={arity}),\n"
        f"{rt_line}"
        "        )\n"
    )


def main() -> None:
    rows = {}
    for line in open(_REPO_ROOT / "tmp/registry-audit/tcl.rust.jsonl"):
        d = json.loads(line)
        if d["name"] in WANT:
            rows[d["name"]] = d

    modules = []
    for cmd, mod in WANT.items():
        (PY_DIR / f"{mod}.py").write_text(file_text(rows[cmd]))
        modules.append(mod)

    # Wire modules into the __init__ import tuple (insert before the
    # closing `)` of `from . import (`).
    text = INIT.read_text()
    m = re.search(r"from \. import \(\n(.*?)\n\)", text, re.S)
    block = m.group(1)
    existing = set(re.findall(r"^\s*(\w+),", block, re.M))
    add = "".join(f"    {mod},  # noqa: F401\n" for mod in sorted(modules) if mod not in existing)
    new_block = block + "\n" + add.rstrip("\n")
    text = text[: m.start(1)] + new_block + text[m.end(1) :]
    INIT.write_text(text)
    print(f"added {len(modules)} python command modules:", sorted(modules))


if __name__ == "__main__":
    main()

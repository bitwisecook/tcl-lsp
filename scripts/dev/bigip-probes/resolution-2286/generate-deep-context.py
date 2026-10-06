#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Generate byte-controlled parser probes for five BIG-IP Tcl contexts."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def decode_source(text: str) -> bytes:
    out = bytearray()
    i = 0
    while i < len(text):
        if text[i] != "\\":
            out.extend(text[i].encode("ascii"))
            i += 1
            continue
        i += 1
        if i == len(text):
            out.append(0x5C)
            break
        escaped = text[i]
        out.extend(
            {
                "n": b"\n",
                "r": b"\r",
                "\\": b"\\",
                "{": b"{",
                "}": b"}",
                '"': b'"',
            }.get(escaped, ("\\" + escaped).encode("ascii"))
        )
        i += 1
    return bytes(out)


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--run", required=True)
parser.add_argument("--canonical", required=True, type=Path)
parser.add_argument("--additional", required=True, type=Path)
parser.add_argument("--out", required=True, type=Path)
args = parser.parse_args()
if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]{0,15}", args.run):
    parser.error("invalid run")
args.out.mkdir(parents=True, exist_ok=False)

cases = []
for path in (args.canonical, args.additional):
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith("#"):
            continue
        case, category, source = raw.split("\t")
        data = decode_source(source)
        cases.append((case, category, data))

prefix = f"__tcl_lsp_probe_2286_{args.run}_deep"


def rows_for(selected):
    return " \\\n".join(
        f"    {{{case}}} {{{category}}} {{{data.hex()}}}"
        for case, category, data in selected
    )


def full_body(context: str, emit: str) -> str:
    rows = rows_for(cases)
    return f"""set ::probe_cases [list \\
{rows}]
foreach {{cid category source_hex}} $::probe_cases {{
    set src [binary format H* $source_hex]
    set ::m unset
    set rc [catch {{uplevel #0 $src}} value]
    binary scan $value H* value_hex
    {emit} "R2286DEEP|{args.run}|{context}|case=$cid|category=$category|source_hex=$source_hex|rc=$rc|value_hex=$value_hex"
}}
set tpl UNSET
if {{[info exists tcl_patchLevel]}} {{ set tpl $tcl_patchLevel }}
set tvcmd tmsh::version
if {{[catch {{eval $tvcmd}} tv]}} {{ set tv n/a }}
set plat [lsort [array names tcl_platform]]
{emit} "R2286DEEP|{args.run}|{context}|REPORTED|patchlevel=[info patchlevel]|tclversion=[info tclversion]|tcl_patchLevel=$tpl|tmshversion=$tv|ncommands=[llength [info commands]]|platform_keys=$plat"
"""


def irule_body(selected, chunk: int) -> str:
    rows = rows_for(selected)
    return f"""set probe_cases [list \\
{rows}]
set acc {{}}
foreach {{cid category source_hex}} $probe_cases {{
    set src [binary format H* $source_hex]
    set ::m unset
    set rc [catch {{uplevel #0 $src}} value]
    binary scan $value H* value_hex
    lappend acc "$cid,$rc,[string range $value_hex 0 119]"
}}
log local0. "R2286DEEP|{args.run}|TmmIRule|chunk={chunk}|[join $acc ;]"
"""


irule_rules = []
irule_names = []
for chunk, start in enumerate(range(0, len(cases), 10)):
    name = f"/Common/{prefix}_irule_{chunk}"
    irule_names.append(name)
    irule_rules.append(
        f"ltm rule {name} {{\nwhen RULE_INIT {{\n{irule_body(cases[start : start + 10], chunk)}\n}}\n}}\n"
    )
files = {
    "irule.conf": "".join(irule_rules),
    "cli.conf": f"cli script /Common/{prefix}_cli {{\nproc script::run {{}} {{\n{full_body('TmshCliScript', 'puts')}\n}}\n}}\n",
    "iapp.conf": "sys application template /Common/"
    + prefix
    + "_iapp {\n  actions {\n    definition {\n      implementation {\n"
    + full_body("IAppImplementation", "puts")
    + "\n      }\n      presentation {\n      }\n    }\n  }\n}\n",
    "iapp-service.conf": f"sys application service /Common/{prefix}_iapp_service {{\n  template /Common/{prefix}_iapp\n}}\n",
    "icall.conf": "sys icall script /Common/"
    + prefix
    + "_icall {\n  definition {\n"
    + full_body("ICallScript", "puts")
    + "\n  }\n}\nsys icall handler triggered /Common/"
    + prefix
    + "_icall_handler {\n  script /Common/"
    + prefix
    + "_icall\n  status active\n  subscriptions { only { event-name "
    + args.run.upper()
    + "_DEEP } }\n}\n",
    "host.tcl": full_body("HostShellTcl", "puts"),
}
manifest = {"run": args.run, "cases": [], "fixtures": [], "irule_names": irule_names}
for case, category, data in cases:
    manifest["cases"].append(
        {"id": case, "category": category, "source_hex": data.hex(), "size": len(data)}
    )
for name, text in files.items():
    data = text.encode("ascii")
    (args.out / name).write_bytes(data)
    manifest["fixtures"].append(
        {"file": name, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    )
(args.out / "manifest.json").write_text(
    json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
)
print(f"generated {len(cases)} cases in {len(files)} wrappers")

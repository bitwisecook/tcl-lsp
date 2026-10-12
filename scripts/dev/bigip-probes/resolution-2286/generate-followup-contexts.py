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

"""Transport one exact follow-up payload into non-TMM BIG-IP Tcl contexts."""

import argparse
import hashlib
import json
import re
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--run", required=True)
parser.add_argument("--payload", required=True, type=Path)
parser.add_argument("--out", required=True, type=Path)
parser.add_argument("--source-commit", required=True)
args = parser.parse_args()
if not re.fullmatch(r"[A-Za-z][A-Za-z0-9]{0,15}", args.run):
    parser.error("run must be ASCII alphanumeric and start with a letter")
if args.out.exists():
    parser.error("output directory already exists")

payload = args.payload.read_bytes()
if not payload.isascii() or b"\x00" in payload or b"\r" in payload:
    parser.error("payload must be ASCII, NUL-free, and LF-only")
args.out.mkdir(parents=True)

prefix = f"__tcl_lsp_2286_{args.run}_contexts"
payload_hex = payload.hex()


def body(context: str, emit: str) -> str:
    return f"""set payload [binary format H* {payload_hex}]
set rc [catch {{eval $payload}} result]
binary scan $result H* result_hex
{emit} "R2286FOLLOWUP|{args.run}|{context}|payload_sha256={hashlib.sha256(payload).hexdigest()}|rc=$rc|result_hex=$result_hex"
"""


files = {
    "cli.conf": (
        f"cli script /Common/{prefix}_cli {{\n"
        "proc script::run {} {\n"
        f"{body('TmshCliScript', 'puts')}"
        "}\n}\n"
    ),
    "iapp.conf": (
        f"sys application template /Common/{prefix}_iapp {{\n"
        "  actions {\n"
        "    definition {\n"
        "      implementation {\n"
        f"{body('IAppImplementation', 'puts')}"
        "      }\n"
        "      presentation {\n"
        "      }\n"
        "    }\n"
        "  }\n"
        "}\n"
    ),
    "iapp-service.conf": (
        f"sys application service /Common/{prefix}_iapp_service {{\n"
        f"  template /Common/{prefix}_iapp\n"
        "}\n"
    ),
    "icall.conf": (
        f"sys icall script /Common/{prefix}_icall {{\n"
        "  definition {\n"
        f"{body('ICallScript', 'puts')}"
        "  }\n"
        "}\n"
        f"sys icall handler triggered /Common/{prefix}_icall_handler {{\n"
        f"  script /Common/{prefix}_icall\n"
        "  status active\n"
        f"  subscriptions {{ only {{ event-name {args.run.upper()}_FOLLOWUP }} }}\n"
        "}\n"
    ),
}

manifest = {
    "run": args.run,
    "source_commit": args.source_commit,
    "payload": {
        "file": args.payload.name,
        "size": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "hex": payload_hex,
        "ascii": payload.isascii(),
        "contains_nul": b"\x00" in payload,
        "contains_cr": b"\r" in payload,
    },
    "fixtures": [],
}
for name, text in files.items():
    data = text.encode("ascii")
    (args.out / name).write_bytes(data)
    manifest["fixtures"].append(
        {"file": name, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    )
(args.out / "payload.tcl").write_bytes(payload)
(args.out / "manifest.json").write_text(
    json.dumps(manifest, indent=2) + "\n", encoding="ascii"
)

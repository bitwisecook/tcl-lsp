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

"""Extract and wrap the exact lexical-byte-class appliance fixtures."""

import argparse
import hashlib
import json
import re
from pathlib import Path


EXPECTED_PAYLOAD = "af1d6366e57927bc72cd756bf8804e1a1713de958561f30258876603bc120ae9"
EXPECTED_RULE_BODY = "49b3c8dcd43221031288595afc53f9cc4642fb714e5a57b123b15d3613d91a40"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def ascii_lf(text: str) -> bytes:
    data = text.encode("ascii")
    if b"\r" in data or b"\x00" in data:
        raise ValueError("generated source must be ASCII, NUL-free, and LF-only")
    return data


def replace_context(script: bytes, context: str) -> bytes:
    needle = b"set r2286_lbc_context tmsh_cli\n"
    if script.count(needle) != 1:
        raise ValueError("non-TMM context marker is not unique")
    return script.replace(
        needle, f"set r2286_lbc_context {context}\n".encode("ascii"), 1
    )


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--out", required=True, type=Path)
parser.add_argument("--source-commit", required=True)
parser.add_argument("--vip", default="192.168.9.24")
parser.add_argument("--vip-port", default=18860, type=int)
parser.add_argument("--backend", default="192.168.9.80")
parser.add_argument("--backend-port", default=18760, type=int)
args = parser.parse_args()
if args.out.exists():
    parser.error("output directory already exists")
for port in (args.vip_port, args.backend_port):
    if not 1024 <= port <= 65535:
        parser.error("ports must be 1024..65535")

here = Path(__file__).resolve().parent
request = here / "LEXICAL_BYTE_CLASS_CHECKS.md"
request_bytes = request.read_bytes()
blocks = re.findall(rb"```tcl\n(.*?)```\n", request_bytes, re.DOTALL)
if len(blocks) != 3:
    raise SystemExit(f"expected three Tcl blocks, found {len(blocks)}")
payload, rule_body, non_tmm = blocks
if digest(payload) != EXPECTED_PAYLOAD:
    raise SystemExit("payload digest does not match the request")
if digest(rule_body) != EXPECTED_RULE_BODY:
    raise SystemExit("full rule-body digest does not match the request")
for label, data in (
    ("payload", payload),
    ("rule body", rule_body),
    ("non-TMM", non_tmm),
):
    if not data.isascii() or b"\r" in data or b"\x00" in data:
        raise SystemExit(f"{label} is not ASCII/LF source")

args.out.mkdir(parents=True)
sources = args.out / "sources"
sources.mkdir()
contexts = args.out / "contexts"
contexts.mkdir()

prefix = "__tcl_lsp_2286_r2286lex1"
common = f"/Common/{prefix}"
(sources / "payload.tcl").write_bytes(payload)
(sources / "irule-body.tcl").write_bytes(rule_body)
(sources / "non-tmm-tmsh-cli.tcl").write_bytes(non_tmm)

irule = ascii_lf(f"ltm rule {common}_lbc {{\n") + rule_body + b"}\n"
(sources / "irule.conf").write_bytes(irule)

identity = ascii_lf(
    f"""ltm rule {common}_identity {{
    when RULE_INIT {{
        log local0. "R2286LBCIDENT|run=r2286lex1|ctx=RULE_INIT|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CLIENT_ACCEPTED {{
        log local0. "R2286LBCIDENT|run=r2286lex1|ctx=CLIENT_ACCEPTED|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|session=[IP::client_addr]:[TCP::client_port]"
    }}
    when HTTP_REQUEST {{
        if {{[HTTP::path] eq "/r2286-identity"}} {{
            HTTP::respond 200 content [list r2286lex1 [TMM::cmp_group] [TMM::cmp_unit] [IP::client_addr] [TCP::client_port]] "Content-Type" "text/plain" "Connection" "close"
        }}
    }}
}}
"""
)
(sources / "identity.conf").write_bytes(identity)

lab = ascii_lf(
    f"""ltm pool {common}_pool {{
    members {{ /Common/{args.backend}:{args.backend_port} {{ address {args.backend} }} }}
}}
ltm virtual {common}_vs {{
    destination {args.vip}:{args.vip_port}
    mask 255.255.255.255
    ip-protocol tcp
    source 0.0.0.0/0
    profiles {{ /Common/tcp {{ }} /Common/http {{ }} }}
    pool {common}_pool
    rules {{ {common}_identity }}
    source-address-translation {{ type automap }}
    cmp-enabled yes
}}
"""
)
(sources / "lab.conf").write_bytes(lab)

cli_name = f"{common}_cli"
cli = (
    ascii_lf(f"cli script {cli_name} {{\nproc script::run {{}} {{\n")
    + non_tmm
    + b"}\n}\n"
)
(contexts / "cli.conf").write_bytes(cli)

iapp_body = replace_context(non_tmm, "iapp_implementation")
iapp = (
    ascii_lf(
        f"sys application template {common}_iapp {{\n"
        "  actions {\n"
        "    definition {\n"
        "      implementation {\n"
    )
    + iapp_body
    + ascii_lf("      }\n      presentation {\n      }\n    }\n  }\n}\n")
)
iapp_service = ascii_lf(
    f"sys application service {common}_iapp_service {{\n  template {common}_iapp\n}}\n"
)
(contexts / "iapp.conf").write_bytes(iapp)
(contexts / "iapp-service.conf").write_bytes(iapp_service)

icall_body = replace_context(non_tmm, "icall_triggered")
icall = (
    ascii_lf(f"sys icall script {common}_icall {{\n  definition {{\n")
    + icall_body
    + ascii_lf(
        "  }\n}\n"
        f"sys icall handler triggered {common}_icall_handler {{\n"
        f"  script {common}_icall\n"
        "  status active\n"
        "  subscriptions { only { event-name R2286LEX1_LBC } }\n"
        "}\n"
    )
)
(contexts / "icall.conf").write_bytes(icall)

manifest = {
    "run": "r2286lex1",
    "source_commit": args.source_commit,
    "request": str(request.relative_to(here.parent.parent.parent.parent)),
    "request_sha256": digest(request_bytes),
    "vip": f"{args.vip}:{args.vip_port}",
    "backend": f"{args.backend}:{args.backend_port}",
    "expected": {
        "payload_sha256": EXPECTED_PAYLOAD,
        "rule_body_sha256": EXPECTED_RULE_BODY,
        "rows_per_execution": 1044,
    },
    "files": [],
}
for path in sorted(item for item in args.out.rglob("*") if item.is_file()):
    manifest["files"].append(
        {
            "file": str(path.relative_to(args.out)),
            "sha256": digest(path.read_bytes()),
            "size": path.stat().st_size,
        }
    )
(args.out / "manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="ascii"
)
hashes = []
for path in sorted(item for item in args.out.rglob("*") if item.is_file()):
    if path.name == "SHA256SUMS":
        continue
    hashes.append(f"{digest(path.read_bytes())}  {path.relative_to(args.out)}")
(args.out / "SHA256SUMS").write_text("\n".join(hashes) + "\n", encoding="ascii")
print(json.dumps(manifest, indent=2, sort_keys=True))

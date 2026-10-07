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

"""Generate the reproducible TLS/APM/ASM/HTTP cross-module event probe."""

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
from pathlib import Path


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--run", required=True)
parser.add_argument("--out", required=True, type=Path)
parser.add_argument("--vip", required=True)
parser.add_argument("--vip-port", required=True, type=int)
parser.add_argument("--backend", required=True)
parser.add_argument("--backend-port", required=True, type=int)
parser.add_argument("--clientssl", required=True)
parser.add_argument("--access-profile", required=True)
parser.add_argument("--asm-policy", required=True)
parser.add_argument("--client-cert-rule", required=True)
parser.add_argument("--apm-rule", required=True)
parser.add_argument("--source-commit")
args = parser.parse_args()

if not re.fullmatch(r"[A-Za-z][A-Za-z0-9]{0,15}", args.run):
    parser.error("run must be ASCII alphanumeric and start with a letter")
if args.out.exists():
    parser.error("output directory already exists")
vip = ipaddress.IPv4Address(args.vip)
backend = ipaddress.IPv4Address(args.backend)
for port in (args.vip_port, args.backend_port):
    if not 1024 <= port <= 65535:
        parser.error("ports must be 1024..65535")
source_commit = args.source_commit or subprocess.run(
    ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
).stdout.strip()
prefix = f"__tcl_lsp_evtflow_{args.run.lower()}_cross"
common = f"/Common/{prefix}"
args.out.mkdir(parents=True)
manifest = {
    "schema": 1,
    "run": args.run,
    "source_commit": source_commit,
    "encoding": "ASCII",
    "line_endings": "LF",
    "vip": str(vip),
    "vip_port": args.vip_port,
    "backend": str(backend),
    "backend_port": args.backend_port,
    "references": {
        "clientssl": args.clientssl,
        "access_profile": args.access_profile,
        "asm_policy": args.asm_policy,
        "client_cert_rule": args.client_cert_rule,
        "apm_rule": args.apm_rule,
    },
    "files": [],
}


def write(name: str, source: str, kind: str) -> None:
    data = source.encode("ascii")
    if b"\r" in data or b"\x00" in data or not data.endswith(b"\n"):
        raise ValueError(f"{name} is not canonical ASCII/LF source")
    path = args.out / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    manifest["files"].append(
        {"file": name, "kind": kind, "size": len(data), "sha256": digest(data)}
    )


def log(event: str, phase: str, extra: str = "") -> str:
    fields = (
        f"EVTFLOW3|run={args.run}|event={event}|phase={phase}"
        "|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    )
    if extra:
        fields += f"|{extra}"
    return f'        log local0. "{fields}"'


write(
    "rules/10-early.conf",
    f"""ltm rule {common}_early {{
    when RULE_INIT {{
{log("RULE_INIT", "early")}
    }}
    when CLIENT_ACCEPTED priority 10 {{
{log("CLIENT_ACCEPTED", "early", "client=[IP::client_addr]:[TCP::client_port]")}
    }}
    when HTTP_REQUEST priority 10 {{
        set evtflow3_id [HTTP::header value X-Evtflow-Request]
{log("HTTP_REQUEST", "early", "id=$evtflow3_id|uri=[HTTP::uri]")}
    }}
}}
""",
    "irule",
)

write(
    "rules/100-security.conf",
    f"""ltm rule {common}_security {{
    when RULE_INIT {{
{log("RULE_INIT", "security")}
    }}
    when HTTP_REQUEST priority 100 {{
        set evtflow3_id [HTTP::header value X-Evtflow-Request]
        ASM::enable {args.asm_policy}
{log("HTTP_REQUEST", "security", f"id=$evtflow3_id|asm={args.asm_policy}")}
    }}
    when ASM_REQUEST_DONE {{
{log("ASM_REQUEST_DONE", "security", "id=$evtflow3_id|status=[ASM::status]|count=[ASM::violation count]|names=[ASM::violation names]")}
    }}
    when ASM_REQUEST_BLOCKING {{
{log("ASM_REQUEST_BLOCKING", "security", "id=$evtflow3_id|policy=[ASM::policy]")}
    }}
}}
""",
    "irule",
)

write(
    "rules/900-late.conf",
    f"""ltm rule {common}_late {{
    when RULE_INIT {{
{log("RULE_INIT", "late")}
    }}
    when HTTP_REQUEST priority 900 {{
        set evtflow3_id [HTTP::header value X-Evtflow-Request]
{log("HTTP_REQUEST", "late", "id=$evtflow3_id|uri=[HTTP::uri]")}
    }}
    when HTTP_REQUEST_RELEASE {{
{log("HTTP_REQUEST_RELEASE", "late", "id=$evtflow3_id|uri=[HTTP::uri]")}
    }}
    when SERVER_CONNECTED {{
{log("SERVER_CONNECTED", "late", "id=$evtflow3_id|server=[IP::server_addr]:[TCP::server_port]")}
    }}
    when HTTP_REQUEST_SEND {{
{log("HTTP_REQUEST_SEND", "late", "id=$evtflow3_id|uri=[HTTP::uri]")}
    }}
    when HTTP_RESPONSE priority 100 {{
{log("HTTP_RESPONSE", "late", "id=$evtflow3_id|status=[HTTP::status]")}
    }}
    when HTTP_RESPONSE_RELEASE {{
{log("HTTP_RESPONSE_RELEASE", "late", "id=$evtflow3_id|status=[HTTP::status]")}
    }}
    when CLIENT_CLOSED {{
        set close_id unset
        if {{ [info exists evtflow3_id] }} {{ set close_id $evtflow3_id }}
{log("CLIENT_CLOSED", "late", "id=$close_id")}
    }}
}}
""",
    "irule",
)

write(
    "config.conf",
    f"""ltm pool {common}_pool {{
    members {{
        {backend}:{args.backend_port} {{ address {backend} }}
    }}
}}
ltm virtual {common}_vs {{
    destination {vip}:{args.vip_port}
    ip-protocol tcp
    mask 255.255.255.255
    pool {common}_pool
    profiles {{
        {args.access_profile} {{ }}
        {args.clientssl} {{ context clientside }}
        /Common/http {{ }}
        /Common/tcp {{ }}
        /Common/websecurity {{ }}
    }}
    rules {{
        {common}_early
        {common}_security
        {common}_late
        {args.client_cert_rule}
        {args.apm_rule}
    }}
    source 0.0.0.0/0
    source-address-translation {{ type automap }}
}}
""",
    "tmsh-config",
)

manifest_data = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode("ascii")
(args.out / "manifest.json").write_bytes(manifest_data)
rows = [f"{row['sha256']}  {row['file']}" for row in manifest["files"]]
rows.append(f"{digest(manifest_data)}  manifest.json")
(args.out / "SHA256SUMS").write_text("\n".join(rows) + "\n", encoding="ascii")

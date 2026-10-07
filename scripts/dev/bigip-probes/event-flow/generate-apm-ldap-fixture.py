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

"""Generate the reproducible APM HTTP-basic-to-LDAP event-flow probe."""

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
parser.add_argument("--ldap", required=True)
parser.add_argument("--ldap-port", required=True, type=int)
parser.add_argument("--clientssl", required=True)
parser.add_argument("--source-commit")
args = parser.parse_args()

if not re.fullmatch(r"[A-Za-z][A-Za-z0-9]{0,15}", args.run):
    parser.error("run must be ASCII alphanumeric and start with a letter")
if args.out.exists():
    parser.error("output directory already exists")
vip = ipaddress.IPv4Address(args.vip)
backend = ipaddress.IPv4Address(args.backend)
ldap = ipaddress.IPv4Address(args.ldap)
for port in (args.vip_port, args.backend_port, args.ldap_port):
    if not 1024 <= port <= 65535:
        parser.error("ports must be 1024..65535")
source_commit = (
    args.source_commit
    or subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
)
prefix = f"__tcl_lsp_evtflow_{args.run.lower()}_ldap"
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
    "ldap": str(ldap),
    "ldap_port": args.ldap_port,
    "clientssl": args.clientssl,
    "generated_test_credentials": {
        "bind_dn": "cn=admin,dc=bitwisecook,dc=org",
        "bind_password": "evtflow-admin-2026",
        "username": "evtflow",
        "password": "evtflow-pass-2026",
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


write(
    "rule.conf",
    f"""ltm rule {common}_trace {{
    when RULE_INIT {{
        log local0. "EVTFLOW4|run={args.run}|event=RULE_INIT|probe=apm-ldap|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CLIENT_ACCEPTED {{
        set evtflow4_id "[IP::client_addr]:[TCP::client_port]"
        log local0. "EVTFLOW4|run={args.run}|event=CLIENT_ACCEPTED|probe=apm-ldap|id=$evtflow4_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_REQUEST {{
        if {{ [HTTP::header exists X-Evtflow-Request] }} {{
            set evtflow4_id [HTTP::header value X-Evtflow-Request]
        }}
        log local0. "EVTFLOW4|run={args.run}|event=HTTP_REQUEST|probe=apm-ldap|id=$evtflow4_id|uri=[HTTP::uri]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when ACCESS_SESSION_STARTED {{
        log local0. "EVTFLOW4|run={args.run}|event=ACCESS_SESSION_STARTED|probe=apm-ldap|id=$evtflow4_id|sid=[ACCESS::session sid]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when ACCESS_POLICY_AGENT_EVENT {{
        set username [ACCESS::session data get session.logon.last.username]
        set authresult [ACCESS::session data get session.ldap.last.authresult]
        log local0. "EVTFLOW4|run={args.run}|event=ACCESS_POLICY_AGENT_EVENT|probe=apm-ldap|id=$evtflow4_id|agent=[ACCESS::policy agent_id]|username=$username|authresult=$authresult|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when ACCESS_POLICY_COMPLETED {{
        set username [ACCESS::session data get session.logon.last.username]
        set authresult [ACCESS::session data get session.ldap.last.authresult]
        log local0. "EVTFLOW4|run={args.run}|event=ACCESS_POLICY_COMPLETED|probe=apm-ldap|id=$evtflow4_id|result=[ACCESS::policy result]|username=$username|authresult=$authresult|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when ACCESS_ACL_ALLOWED {{
        log local0. "EVTFLOW4|run={args.run}|event=ACCESS_ACL_ALLOWED|probe=apm-ldap|id=$evtflow4_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when SERVER_CONNECTED {{
        log local0. "EVTFLOW4|run={args.run}|event=SERVER_CONNECTED|probe=apm-ldap|id=$evtflow4_id|peer=[IP::server_addr]:[TCP::server_port]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
""",
    "irule",
)

write(
    "config.conf",
    f"""apm aaa ldap {common}_server {{
    address {ldap}
    admin-dn "cn=admin,dc=bitwisecook,dc=org"
    admin-encrypted-password "!evtflow-admin-2026"
    base-dn "dc=bitwisecook,dc=org"
    port {args.ldap_port}
    schema-attr {{
        group-member member
        group-member-value dn
        group-memberof memberOf
        group-object-class groupOfNames
        user-memberof memberOf
        user-object-class inetOrgPerson
    }}
    use-pool disabled
}}
apm policy customization-group {common}_logon_cg {{
    source standard
    type logon
}}
apm policy agent logon-page {common}_logon_ag {{
    basic-auth-realm "tcl-lsp event-flow LDAP"
    customization-group {common}_logon_cg
    http-401-auth-level basic
    split-username false
    type 401
}}
apm policy agent aaa-ldap {common}_auth_ag {{
    max-logon-attempt 1
    server {common}_server
    type auth
    user-dn "uid=%{{session.logon.last.username}},ou=people,dc=bitwisecook,dc=org"
}}
apm policy agent irule-event {common}_success_ag {{
    expect-data http
    id {args.run}_ldap_success
}}
apm policy agent irule-event {common}_failure_ag {{
    expect-data http
    id {args.run}_ldap_failure
}}
apm policy agent ending-allow {common}_allow_ag {{ }}
apm policy agent ending-deny {common}_deny_ag {{ }}
apm policy policy-item {common}_start {{
    caption Start
    rules {{ {{ caption fallback next-item {common}_logon }} }}
}}
apm policy policy-item {common}_logon {{
    agents {{ {common}_logon_ag {{ type logon-page }} }}
    caption "HTTP Basic logon"
    color 1
    item-type action
    rules {{ {{ caption fallback next-item {common}_auth }} }}
}}
apm policy policy-item {common}_auth {{
    agents {{ {common}_auth_ag {{ type aaa-ldap }} }}
    caption "LDAP authentication"
    color 1
    item-type action
    rules {{
        {{ caption Successful expression "expr {{ [mcget {{session.ldap.last.authresult}}] == 1 }}" next-item {common}_success }}
        {{ caption fallback next-item {common}_failure }}
    }}
}}
apm policy policy-item {common}_success {{
    agents {{ {common}_success_ag {{ type irule-event }} }}
    caption "LDAP success event"
    color 1
    item-type action
    rules {{ {{ caption fallback next-item {common}_allow }} }}
}}
apm policy policy-item {common}_failure {{
    agents {{ {common}_failure_ag {{ type irule-event }} }}
    caption "LDAP failure event"
    color 2
    item-type action
    rules {{ {{ caption fallback next-item {common}_deny }} }}
}}
apm policy policy-item {common}_allow {{
    agents {{ {common}_allow_ag {{ type ending-allow }} }}
    caption Allow
    color 1
    item-type ending
}}
apm policy policy-item {common}_deny {{
    agents {{ {common}_deny_ag {{ type ending-deny }} }}
    caption Deny
    color 2
    item-type ending
}}
apm policy access-policy {common}_policy {{
    default-ending {common}_deny
    items {{
        {common}_start {{ priority 1 }}
        {common}_logon {{ priority 2 }}
        {common}_auth {{ priority 3 }}
        {common}_success {{ priority 4 }}
        {common}_failure {{ priority 5 }}
        {common}_allow {{ priority 6 }}
        {common}_deny {{ priority 7 }}
    }}
    start-item {common}_start
}}
apm profile access {common}_access {{
    defaults-from /Common/access
    access-policy {common}_policy
    generation-action increment
    max-failure-delay 0
    min-failure-delay 0
}}
ltm pool {common}_pool {{
    members {{ {backend}:{args.backend_port} {{ address {backend} }} }}
}}
ltm virtual {common}_vs {{
    destination {vip}:{args.vip_port}
    ip-protocol tcp
    mask 255.255.255.255
    pool {common}_pool
    profiles {{
        {common}_access {{ }}
        {args.clientssl} {{ context clientside }}
        /Common/http {{ }}
        /Common/tcp {{ }}
    }}
    rules {{ {common}_trace }}
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

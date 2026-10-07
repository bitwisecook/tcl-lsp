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

"""Generate the retained multi-source APM authentication laboratory."""

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
parser.add_argument("--ad", required=True)
parser.add_argument("--oidc", required=True)
parser.add_argument("--oidc-port", required=True, type=int)
parser.add_argument("--clientssl", required=True)
parser.add_argument("--keycloak-cert", required=True, type=Path)
parser.add_argument("--source-commit")
args = parser.parse_args()

if not re.fullmatch(r"[A-Za-z][A-Za-z0-9]{0,15}", args.run):
    parser.error("run must be ASCII alphanumeric and start with a letter")
if args.out.exists():
    parser.error("output directory already exists")
for value in (args.vip, args.backend, args.ldap, args.ad, args.oidc):
    ipaddress.IPv4Address(value)
for port in (args.vip_port, args.backend_port, args.ldap_port, args.oidc_port):
    if not 1 <= port <= 65535:
        parser.error("invalid port")
source_commit = (
    args.source_commit
    or subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
)
prefix = f"__tcl_lsp_evtflow_{args.run.lower()}_complex"
common = f"/Common/{prefix}"
args.out.mkdir(parents=True)
manifest = {
    "schema": 1,
    "run": args.run,
    "source_commit": source_commit,
    "encoding": "ASCII except copied PEM certificate",
    "line_endings": "LF",
    "vip": f"{args.vip}:{args.vip_port}",
    "backend": f"{args.backend}:{args.backend_port}",
    "ldap": f"{args.ldap}:{args.ldap_port}",
    "ad": args.ad,
    "oidc": f"{args.oidc}:{args.oidc_port}",
    "generated_test_credentials": {
        "ad_username": "evtflow-ad",
        "ad_password": "Evtflow-Ad-User-2026!",
        "ldap_username": "evtflow",
        "ldap_password": "evtflow-pass-2026",
        "oidc_username": "evtflow-oidc",
        "oidc_password": "Evtflow-Oidc-User-2026!",
        "oidc_client_secret": "evtflow-oidc-client-secret-2026",
    },
    "files": [],
}


def write(name: str, source: str, kind: str) -> None:
    data = source.encode("ascii")
    if b"\r" in data or b"\x00" in data or not data.endswith(b"\n"):
        raise ValueError(f"{name} is not canonical ASCII/LF source")
    (args.out / name).write_bytes(data)
    manifest["files"].append(
        {"file": name, "kind": kind, "size": len(data), "sha256": digest(data)}
    )


cert_data = args.keycloak_cert.read_bytes()
if b"-----BEGIN CERTIFICATE-----" not in cert_data or b"\r" in cert_data:
    parser.error("Keycloak certificate is not canonical PEM/LF")
(args.out / "keycloak-ca.crt").write_bytes(cert_data)
manifest["files"].append(
    {
        "file": "keycloak-ca.crt",
        "kind": "generated-test-certificate",
        "size": len(cert_data),
        "sha256": digest(cert_data),
    }
)

write(
    "rule.conf",
    f'''ltm rule {common}_trace {{
    when RULE_INIT {{
        log local0. "EVTFLOW5|run={args.run}|event=RULE_INIT|probe=complex-apm|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CLIENT_ACCEPTED {{
        set evtflow5_id "[IP::client_addr]:[TCP::client_port]"
        log local0. "EVTFLOW5|run={args.run}|event=CLIENT_ACCEPTED|id=$evtflow5_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_REQUEST {{
        if {{ [HTTP::header exists X-Evtflow-Request] }} {{ set evtflow5_id [HTTP::header value X-Evtflow-Request] }}
        set required public
        if {{ [HTTP::path] starts_with "/ad" }} {{ set required ad }}
        if {{ [HTTP::path] starts_with "/ldap" }} {{ set required ldap }}
        if {{ [HTTP::path] starts_with "/oidc" }} {{ set required oidc }}
        if {{ [HTTP::path] starts_with "/mfa" }} {{ set required mfa }}
        set granted [ACCESS::session data get session.custom.evtflow_scope]
        log local0. "EVTFLOW5|run={args.run}|event=HTTP_REQUEST|id=$evtflow5_id|uri=[HTTP::uri]|required=$required|granted=$granted|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
        if {{ $required ne "public" && $granted ne "" && $granted ne $required }} {{
            log local0. "EVTFLOW5|run={args.run}|event=SCOPE_DENY|id=$evtflow5_id|required=$required|granted=$granted|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
            HTTP::respond 403 content "scope mismatch: required=$required granted=$granted\\n" noserver
            event disable all
        }}
    }}
    when ACCESS_SESSION_STARTED {{
        log local0. "EVTFLOW5|run={args.run}|event=ACCESS_SESSION_STARTED|id=$evtflow5_id|sid=[ACCESS::session sid]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when ACCESS_POLICY_AGENT_EVENT {{
        set agent [ACCESS::policy agent_id]
        if {{ $agent eq "{args.run}_public_ok" }} {{ ACCESS::session data set session.custom.evtflow_scope public }}
        if {{ $agent eq "{args.run}_ad_ok" }} {{ ACCESS::session data set session.custom.evtflow_scope ad }}
        if {{ $agent eq "{args.run}_ldap_ok" }} {{ ACCESS::session data set session.custom.evtflow_scope ldap }}
        if {{ $agent eq "{args.run}_oidc_ok" }} {{ ACCESS::session data set session.custom.evtflow_scope oidc }}
        if {{ $agent eq "{args.run}_mfa_ok" }} {{ ACCESS::session data set session.custom.evtflow_scope mfa }}
        set otp ""
        if {{ $agent eq "{args.run}_otp_issued" }} {{ set otp [ACCESS::session data get session.otp.assigned.val] }}
        log local0. "EVTFLOW5|run={args.run}|event=ACCESS_POLICY_AGENT_EVENT|id=$evtflow5_id|agent=$agent|otp=$otp|ad=[ACCESS::session data get session.ad.last.authresult]|ldap=[ACCESS::session data get session.ldap.last.authresult]|oauth=[ACCESS::session data get session.oauth.client.last.result]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when ACCESS_POLICY_COMPLETED {{
        log local0. "EVTFLOW5|run={args.run}|event=ACCESS_POLICY_COMPLETED|id=$evtflow5_id|result=[ACCESS::policy result]|scope=[ACCESS::session data get session.custom.evtflow_scope]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when ACCESS_ACL_ALLOWED {{
        log local0. "EVTFLOW5|run={args.run}|event=ACCESS_ACL_ALLOWED|id=$evtflow5_id|scope=[ACCESS::session data get session.custom.evtflow_scope]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when SERVER_CONNECTED {{
        log local0. "EVTFLOW5|run={args.run}|event=SERVER_CONNECTED|id=$evtflow5_id|peer=[IP::server_addr]:[TCP::server_port]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
''',
    "irule",
)

# All objects are deliberately in one load unit so foreign keys are checked
# together. The install script creates the copied CA file object first.
items: list[tuple[str, str, str, str]] = []


def item(name: str, agent: str, agent_type: str, caption: str, rules: str) -> None:
    items.append((name, agent, agent_type, caption, rules))


def next_rule(target: str) -> str:
    return f"{{ {{ caption fallback next-item {common}_{target} }} }}"


item("start", "", "", "Start", next_rule("route"))
item(
    "route",
    "",
    "",
    "Route by landing URI",
    f"""{{
        {{ caption MFA expression "expr {{ [string match \"/mfa*\" [mcget {{session.server.landinguri}}]] }}" next-item {common}_mfa_logon }}
        {{ caption OIDC expression "expr {{ [string match \"/oidc*\" [mcget {{session.server.landinguri}}]] }}" next-item {common}_oidc_auth }}
        {{ caption AD expression "expr {{ [string match \"/ad*\" [mcget {{session.server.landinguri}}]] }}" next-item {common}_ad_logon }}
        {{ caption LDAP expression "expr {{ [string match \"/ldap*\" [mcget {{session.server.landinguri}}]] }}" next-item {common}_ldap_logon }}
        {{ caption fallback next-item {common}_public_event }}
    }}""",
)
item(
    "public_event", "public_event_ag", "irule-event", "Public scope", next_rule("allow")
)
item(
    "ad_logon",
    "ad_logon_ag",
    "logon-page",
    "AD username/password",
    next_rule("ad_auth"),
)
item(
    "ad_auth",
    "ad_auth_ag",
    "aaa-active-directory",
    "Samba AD authentication",
    f"""{{
        {{ caption Successful expression "expr {{ [mcget {{session.ad.last.authresult}}] == 1 }}" next-item {common}_ad_event }}
        {{ caption fallback next-item {common}_deny }}
    }}""",
)
item("ad_event", "ad_event_ag", "irule-event", "AD scope", next_rule("allow"))
item(
    "ldap_logon",
    "ldap_logon_ag",
    "logon-page",
    "LDAP username/password",
    next_rule("ldap_auth"),
)
item(
    "ldap_auth",
    "ldap_auth_ag",
    "aaa-ldap",
    "OpenLDAP authentication",
    f"""{{
        {{ caption Successful expression "expr {{ [mcget {{session.ldap.last.authresult}}] == 1 }}" next-item {common}_ldap_event }}
        {{ caption fallback next-item {common}_deny }}
    }}""",
)
item("ldap_event", "ldap_event_ag", "irule-event", "LDAP scope", next_rule("allow"))
item(
    "oidc_auth",
    "oidc_auth_ag",
    "aaa-oauth",
    "Keycloak OAuth authorization code",
    f"""{{
        {{ caption Successful expression "expr {{ [string length [mcget {{session.oauth.client.last.access_token}}]] > 0 }}" next-item {common}_oidc_event }}
        {{ caption fallback next-item {common}_deny }}
    }}""",
)
item("oidc_event", "oidc_event_ag", "irule-event", "OIDC scope", next_rule("allow"))
item("mfa_logon", "mfa_logon_ag", "logon-page", "MFA AD password", next_rule("mfa_ad"))
item(
    "mfa_ad",
    "mfa_ad_auth_ag",
    "aaa-active-directory",
    "MFA AD authentication",
    f"""{{
        {{ caption Successful expression "expr {{ [mcget {{session.ad.last.authresult}}] == 1 }}" next-item {common}_otp_generate }}
        {{ caption fallback next-item {common}_deny }}
    }}""",
)
item(
    "otp_generate",
    "otp_generate_ag",
    "otp-generate",
    "Generate OTP",
    next_rule("otp_issued"),
)
item(
    "otp_issued",
    "otp_issued_ag",
    "irule-event",
    "Expose generated test OTP",
    next_rule("otp_logon"),
)
item(
    "otp_logon", "otp_logon_ag", "logon-page", "OTP challenge", next_rule("otp_verify")
)
item(
    "otp_verify",
    "otp_verify_ag",
    "otp-verify",
    "Verify OTP",
    f"""{{
        {{ caption Successful expression "expr {{ [mcget {{session.otp.verify.last.authresult}}] == 1 }}" next-item {common}_mfa_event }}
        {{ caption fallback next-item {common}_deny }}
    }}""",
)
item("mfa_event", "mfa_event_ag", "irule-event", "MFA scope", next_rule("allow"))

object_text = []
for priority, (name, agent, agent_type, caption, rules) in enumerate(items, 1):
    agents = (
        f"agents {{ {common}_{agent} {{ type {agent_type} }} }}\n    " if agent else ""
    )
    item_type = "entry" if name == "start" else "action"
    object_text.append(f'''apm policy policy-item {common}_{name} {{
    {agents}caption "{caption}"
    color 1
    item-type {item_type}
    rules {rules}
}}''')

ending_priority = len(items) + 1
objects = "\n".join(object_text)
item_priorities = "\n".join(
    f"        {common}_{name} {{ priority {priority} }}"
    for priority, (name, *_rest) in enumerate(items, 1)
)

write(
    "config.conf",
    f"""apm aaa active-directory {common}_ad_server {{
    domain lab.bitwisecook.org
    domain-controller {args.ad}
    domain-controllers {{ ad.lab.bitwisecook.org {{ ip {args.ad} }} }}
}}
apm aaa ldap {common}_ldap_server {{
    address {args.ldap}
    admin-dn "cn=admin,dc=bitwisecook,dc=org"
    admin-encrypted-password "!evtflow-admin-2026"
    base-dn "dc=bitwisecook,dc=org"
    port {args.ldap_port}
    use-pool disabled
}}
net dns-resolver {common}_resolver {{ route-domain 0 }}
apm aaa oauth-provider {common}_oidc_provider {{
    allow-self-signed-jwk-cert yes
    authentication-uri https://{args.oidc}:{args.oidc_port}/realms/evtflow/protocol/openid-connect/auth
    openid-cfg-uri https://{args.oidc}:{args.oidc_port}/realms/evtflow/.well-known/openid-configuration
    token-uri https://{args.oidc}:{args.oidc_port}/realms/evtflow/protocol/openid-connect/token
    trusted-ca-bundle {common}_keycloak_ca
    type custom
    use-auto-jwt-config true
    userinfo-request-uri https://{args.oidc}:{args.oidc_port}/realms/evtflow/protocol/openid-connect/userinfo
}}
apm aaa oauth-server {common}_oidc_server {{
    client-id evtflow-apm
    client-secret "evtflow-oidc-client-secret-2026"
    client-serverssl-profile-name /Common/serverssl
    dns-resolver-name {common}_resolver
    mode client
    provider-name {common}_oidc_provider
}}
apm aaa oauth-request {common}_auth_request {{
    method get
    parameters {{
        client_id {{ type client-id }}
        redirect_uri {{ type redirect-uri }}
        response_type {{ type response-type }}
        scope {{ type scope }}
    }}
    type auth-redirect-request
}}
apm aaa oauth-request {common}_token_request {{
    method post
    parameters {{
        client_id {{ type client-id }}
        client_secret {{ type client-secret }}
        grant_type {{ type grant-type }}
        redirect_uri {{ type redirect-uri }}
    }}
    type token-request
}}
apm aaa oauth-request {common}_userinfo_request {{
    headers {{ Authorization {{ value "Bearer %{{session.oauth.client.last.access_token}}" }} }}
    method get
    type openid-userinfo-request
}}
apm policy customization-group {common}_logon_cg {{ source standard type logon }}
apm policy customization-group {common}_otp_cg {{ source standard type logon }}
apm policy agent logon-page {common}_ad_logon_ag {{
    customization-group {common}_logon_cg
    fieldtype1 text
    field-type2 password
    post-var-name1 username
    post-var-name2 password
    sess-var-name1 username
    sess-var-name2 password
    type form-based
}}
apm policy agent logon-page {common}_ldap_logon_ag {{
    customization-group {common}_logon_cg
    fieldtype1 text
    field-type2 password
    post-var-name1 username
    post-var-name2 password
    sess-var-name1 username
    sess-var-name2 password
    type form-based
}}
apm policy agent logon-page {common}_mfa_logon_ag {{
    customization-group {common}_logon_cg
    fieldtype1 text
    field-type2 password
    post-var-name1 username
    post-var-name2 password
    sess-var-name1 username
    sess-var-name2 password
    type form-based
}}
apm policy agent logon-page {common}_otp_logon_ag {{
    customization-group {common}_otp_cg
    fieldtype1 none
    field-type2 password
    post-var-name2 password
    sess-var-name2 password
    type form-based
}}
apm policy agent aaa-active-directory {common}_ad_auth_ag {{
    auth-max-logon-attempt 1
    server {common}_ad_server
    type auth
}}
apm policy agent aaa-active-directory {common}_mfa_ad_auth_ag {{
    auth-max-logon-attempt 1
    server {common}_ad_server
    type auth
}}
apm policy agent aaa-ldap {common}_ldap_auth_ag {{
    max-logon-attempt 1
    server {common}_ldap_server
    type auth
    user-dn "uid=%{{session.logon.last.username}},ou=people,dc=bitwisecook,dc=org"
}}
apm policy agent aaa-oauth {common}_oidc_auth_ag {{
    auth-redirect-request {common}_auth_request
    grant-type authorization-code
    openid-connect disabled
    redirection-uri "https://app.bitwisecook.org:{args.vip_port}/oauth/client/redirect"
    scope "openid profile email"
    server {common}_oidc_server
    token-request {common}_token_request
    type client
}}
apm policy agent otp-generate {common}_otp_generate_ag {{ otp-length 6 otp-timeout 300 }}
apm policy agent otp-verify {common}_otp_verify_ag {{ max-logon-attempt 3 otp-source "%{{session.logon.last.password}}" }}
apm policy agent irule-event {common}_public_event_ag {{ expect-data http id {args.run}_public_ok }}
apm policy agent irule-event {common}_ad_event_ag {{ expect-data http id {args.run}_ad_ok }}
apm policy agent irule-event {common}_ldap_event_ag {{ expect-data http id {args.run}_ldap_ok }}
apm policy agent irule-event {common}_oidc_event_ag {{ expect-data http id {args.run}_oidc_ok }}
apm policy agent irule-event {common}_otp_issued_ag {{ expect-data http id {args.run}_otp_issued }}
apm policy agent irule-event {common}_mfa_event_ag {{ expect-data http id {args.run}_mfa_ok }}
apm policy agent ending-allow {common}_allow_ag {{ }}
apm policy agent ending-deny {common}_deny_ag {{ }}
{objects}
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
{item_priorities}
        {common}_allow {{ priority {ending_priority} }}
        {common}_deny {{ priority {ending_priority + 1} }}
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
ltm pool {common}_pool {{ members {{ {args.backend}:{args.backend_port} {{ address {args.backend} }} }} }}
ltm virtual {common}_vs {{
    destination {args.vip}:{args.vip_port}
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

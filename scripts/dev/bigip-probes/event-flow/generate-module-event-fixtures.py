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

"""Generate reproducible APM, ASM, data-event, TLS, WebSocket and MRF probes."""

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
from pathlib import Path


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--run", required=True)
parser.add_argument("--out", required=True, type=Path)
parser.add_argument("--vip", required=True)
parser.add_argument("--backend", required=True)
parser.add_argument("--vip-port-base", required=True, type=int)
parser.add_argument("--backend-port-base", required=True, type=int)
parser.add_argument("--source-commit")
args = parser.parse_args()

if not re.fullmatch(r"[A-Za-z][A-Za-z0-9]{0,15}", args.run):
    parser.error("run must be ASCII alphanumeric and start with a letter")
if args.out.exists():
    parser.error("output directory already exists")
vip = ipaddress.IPv4Address(args.vip)
backend = ipaddress.IPv4Address(args.backend)
if vip == backend or vip.is_loopback or backend.is_loopback:
    parser.error("VIP and backend must be distinct, non-loopback addresses")
for value in (args.vip_port_base, args.backend_port_base):
    if not 1024 <= value <= 65400:
        parser.error("port bases must be 1024..65400")

source_commit = (
    args.source_commit
    or subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
)
prefix = f"__tcl_lsp_evtflow_{args.run.lower()}"
common = f"/Common/{prefix}"
manifest = {
    "schema": 1,
    "run": args.run,
    "source_commit": source_commit,
    "encoding": "ASCII",
    "line_endings": "LF",
    "vip": str(vip),
    "backend": str(backend),
    "vip_port_base": args.vip_port_base,
    "backend_port_base": args.backend_port_base,
    "prefix": prefix,
    "files": [],
}
args.out.mkdir(parents=True)


def write(name: str, source: str, kind: str) -> None:
    data = source.encode("ascii")
    if b"\r" in data or b"\x00" in data or not data.endswith(b"\n"):
        raise ValueError(f"{name} is not canonical ASCII/LF source")
    path = args.out / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    manifest["files"].append(
        {"file": name, "kind": kind, "size": len(data), "sha256": sha256(data)}
    )


def event_log(event: str, extra: str = "") -> str:
    fields = (
        f"EVTFLOW2|run={args.run}|event={event}|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    )
    if extra:
        fields += f"|{extra}"
    return f'        log local0. "{fields}"'


write(
    "rules/tcp-user-data.conf",
    f"""ltm rule {common}_tcp_user_data {{
    when RULE_INIT {{
{event_log("RULE_INIT", "probe=tcp-user-data")}
    }}
    when CLIENT_ACCEPTED {{
        set evtflow2_id [format {{%s:%s}} [IP::client_addr] [TCP::client_port]]
{event_log("CLIENT_ACCEPTED", "probe=tcp-user-data|id=$evtflow2_id")}
        TCP::collect
    }}
    when CLIENT_DATA {{
        binary scan [TCP::payload] H* payload_hex
{event_log("CLIENT_DATA", "probe=tcp-user-data|id=$evtflow2_id|bytes=[TCP::payload length]|hex=$payload_hex")}
        TCP::release
        TCP::notify request
        TCP::collect
    }}
    when USER_REQUEST {{
{event_log("USER_REQUEST", "probe=tcp-user-data|id=$evtflow2_id|side=serverside")}
    }}
    when SERVER_CONNECTED {{
{event_log("SERVER_CONNECTED", "probe=tcp-user-data|id=$evtflow2_id")}
        TCP::collect
    }}
    when SERVER_DATA {{
        binary scan [TCP::payload] H* payload_hex
{event_log("SERVER_DATA", "probe=tcp-user-data|id=$evtflow2_id|bytes=[TCP::payload length]|hex=$payload_hex")}
        TCP::release
        TCP::notify response
        TCP::collect
    }}
    when USER_RESPONSE {{
{event_log("USER_RESPONSE", "probe=tcp-user-data|id=$evtflow2_id|side=clientside")}
    }}
    when CLIENT_CLOSED {{
{event_log("CLIENT_CLOSED", "probe=tcp-user-data|id=$evtflow2_id")}
    }}
}}
""",
    "irule",
)

write(
    "rules/ssl-data.conf",
    f"""ltm rule {common}_ssl_data {{
    when RULE_INIT {{
{event_log("RULE_INIT", "probe=ssl-data")}
    }}
    when CLIENTSSL_HANDSHAKE {{
{event_log("CLIENTSSL_HANDSHAKE", "probe=ssl-data|cipher=[SSL::cipher name]")}
        SSL::collect 1
    }}
    when CLIENTSSL_DATA {{
        binary scan [SSL::payload] H* payload_hex
{event_log("CLIENTSSL_DATA", "probe=ssl-data|bytes=[SSL::payload length]|hex=$payload_hex")}
        SSL::release
    }}
    when SERVERSSL_HANDSHAKE {{
{event_log("SERVERSSL_HANDSHAKE", "probe=ssl-data|cipher=[SSL::cipher name]")}
        SSL::collect 1
    }}
    when SERVERSSL_DATA {{
        binary scan [SSL::payload] H* payload_hex
{event_log("SERVERSSL_DATA", "probe=ssl-data|bytes=[SSL::payload length]|hex=$payload_hex")}
        SSL::release
    }}
}}
""",
    "irule",
)

write(
    "rules/client-cert.conf",
    f"""ltm rule {common}_client_cert {{
    when RULE_INIT {{
{event_log("RULE_INIT", "probe=client-cert")}
    }}
    when CLIENT_ACCEPTED {{
        set evtflow2_cert_renegotiate none
        set evtflow2_http_held 0
    }}
    when CLIENTSSL_CLIENTHELLO {{
{event_log("CLIENTSSL_CLIENTHELLO", "probe=client-cert")}
    }}
    when CLIENTSSL_CLIENTCERT {{
        set cert_count [SSL::cert count]
        set verify_result [SSL::verify_result]
        set subject none
        if {{ $cert_count > 0 }} {{
            set subject [X509::subject [SSL::cert 0]]
        }}
{event_log("CLIENTSSL_CLIENTCERT", "probe=client-cert|count=$cert_count|verify=$verify_result|subject=$subject|renegotiate=$evtflow2_cert_renegotiate")}
    }}
    when CLIENTSSL_HANDSHAKE {{
        set cert_count [SSL::cert count]
        set verify_result [SSL::verify_result]
{event_log("CLIENTSSL_HANDSHAKE", "probe=client-cert|count=$cert_count|verify=$verify_result|mode=[SSL::cert mode]|renegotiate=$evtflow2_cert_renegotiate")}
        if {{ $evtflow2_http_held }} {{
            set evtflow2_http_held 0
            HTTP::release
        }}
    }}
    when HTTP_REQUEST priority 100 {{
        set evtflow2_request_id [HTTP::header value X-Evtflow-Request]
        set cert_count [SSL::cert count]
        set verify_result [SSL::verify_result]
        set path [HTTP::path]
{event_log("HTTP_REQUEST", "probe=client-cert|id=$evtflow2_request_id|path=$path|count=$cert_count|verify=$verify_result|mode=[SSL::cert mode]")}
        if {{ $path starts_with "/cert/request" && $cert_count == 0 }} {{
            set evtflow2_cert_renegotiate request
            set evtflow2_http_held 1
            HTTP::collect
            SSL::session invalidate
            SSL::authenticate always
            SSL::authenticate depth 9
            SSL::cert mode request
            SSL::renegotiate enable
            SSL::renegotiate
            return
        }}
        if {{ $path starts_with "/cert/require" && $cert_count == 0 }} {{
            set evtflow2_cert_renegotiate require
            set evtflow2_http_held 1
            HTTP::collect
            SSL::session invalidate
            SSL::authenticate always
            SSL::authenticate depth 9
            SSL::cert mode require
            SSL::renegotiate enable
            SSL::renegotiate
            return
        }}
        if {{ $path starts_with "/cert/ignore" }} {{
            SSL::cert mode ignore
        }}
        HTTP::header replace X-Evtflow-Cert-Count $cert_count
        HTTP::header replace X-Evtflow-Cert-Verify $verify_result
    }}
}}
""",
    "irule",
)

write(
    "controls/clientssl-alert-unknown-event.conf",
    f"""ltm rule {common}_clientssl_alert_unknown_event {{
    when CLIENTSSL_ALERT {{
{event_log("CLIENTSSL_ALERT", "probe=literal-control")}
    }}
}}
""",
    "literal-control",
)

write(
    "rules/websocket-data.conf",
    f"""ltm rule {common}_websocket_data {{
    when WS_REQUEST {{
{event_log("WS_REQUEST", "probe=websocket")}
    }}
    when WS_RESPONSE {{
{event_log("WS_RESPONSE", "probe=websocket")}
    }}
    when WS_CLIENT_FRAME {{
{event_log("WS_CLIENT_FRAME", "probe=websocket")}
        WS::collect frame
    }}
    when WS_CLIENT_DATA {{
        binary scan [WS::payload] H* payload_hex
{event_log("WS_CLIENT_DATA", "probe=websocket|bytes=[WS::payload length]|hex=$payload_hex")}
        WS::release
    }}
    when WS_CLIENT_FRAME_DONE {{
{event_log("WS_CLIENT_FRAME_DONE", "probe=websocket")}
    }}
    when WS_SERVER_FRAME {{
{event_log("WS_SERVER_FRAME", "probe=websocket")}
        WS::collect frame
    }}
    when WS_SERVER_DATA {{
        binary scan [WS::payload] H* payload_hex
{event_log("WS_SERVER_DATA", "probe=websocket|bytes=[WS::payload length]|hex=$payload_hex")}
        WS::release
    }}
    when WS_SERVER_FRAME_DONE {{
{event_log("WS_SERVER_FRAME_DONE", "probe=websocket")}
    }}
}}
""",
    "irule",
)

write(
    "rules/apm-trace.conf",
    f"""ltm rule {common}_apm_trace {{
    when RULE_INIT {{
{event_log("RULE_INIT", "probe=apm")}
    }}
    when ACCESS_SESSION_STARTED {{
{event_log("ACCESS_SESSION_STARTED", "probe=apm|sid=[ACCESS::session sid]")}
    }}
    when ACCESS_POLICY_AGENT_EVENT {{
        set agent [ACCESS::policy agent_id]
        ACCESS::session data set session.custom.evtflow2_agent $agent
{event_log("ACCESS_POLICY_AGENT_EVENT", "probe=apm|agent=$agent|sid=[ACCESS::session sid]")}
    }}
    when ACCESS_POLICY_COMPLETED {{
{event_log("ACCESS_POLICY_COMPLETED", "probe=apm|result=[ACCESS::policy result]|sid=[ACCESS::session sid]")}
    }}
    when ACCESS_ACL_ALLOWED {{
{event_log("ACCESS_ACL_ALLOWED", "probe=apm|sid=[ACCESS::session sid]")}
    }}
    when ACCESS_ACL_DENIED {{
{event_log("ACCESS_ACL_DENIED", "probe=apm|sid=[ACCESS::session sid]")}
    }}
    when ACCESS_SESSION_CLOSED {{
{event_log("ACCESS_SESSION_CLOSED", "probe=apm")}
    }}
    when HTTP_REQUEST priority 100 {{
{event_log("HTTP_REQUEST", "probe=apm|uri=[HTTP::uri]|internal=[ACCESS::policy uri]")}
    }}
}}
""",
    "irule",
)

write(
    "controls/access-closed-session-sid.conf",
    f"""ltm rule {common}_access_closed_session_sid {{
    when ACCESS_SESSION_CLOSED {{
{event_log("ACCESS_SESSION_CLOSED", "probe=literal-control|sid=[ACCESS::session sid]")}
    }}
}}
""",
    "literal-control",
)

for mode in ("transparent", "blocking"):
    policy = f"{common}_asm_{mode}"
    write(
        f"rules/asm-{mode}.conf",
        f"""ltm rule {common}_asm_{mode}_trace {{
    when RULE_INIT {{
{event_log("RULE_INIT", f"probe=asm|mode={mode}")}
    }}
    when HTTP_REQUEST priority 100 {{
        set evtflow2_asm_id [HTTP::header value X-Evtflow-Request]
        ASM::enable {policy}
{event_log("HTTP_REQUEST", f"probe=asm|mode={mode}|id=$evtflow2_asm_id|uri=[HTTP::uri]")}
    }}
    when ASM_REQUEST_DONE {{
{event_log("ASM_REQUEST_DONE", f"probe=asm|mode={mode}|id=$evtflow2_asm_id|status=[ASM::status]|count=[ASM::violation count]|names=[ASM::violation names]")}
    }}
    when ASM_REQUEST_VIOLATION {{
{event_log("ASM_REQUEST_VIOLATION", f"probe=asm|mode={mode}|id=$evtflow2_asm_id")}
    }}
    when ASM_REQUEST_BLOCKING {{
{event_log("ASM_REQUEST_BLOCKING", f"probe=asm|mode={mode}|id=$evtflow2_asm_id|policy=[ASM::policy]")}
    }}
    when ASM_RESPONSE_VIOLATION {{
{event_log("ASM_RESPONSE_VIOLATION", f"probe=asm|mode={mode}|id=$evtflow2_asm_id")}
    }}
}}
""",
        "irule",
    )

write(
    "rules/mrf-trace.conf",
    f"""ltm rule {common}_mrf_trace {{
    when RULE_INIT {{
{event_log("RULE_INIT", "probe=mrf")}
    }}
    when CLIENT_ACCEPTED {{
        set evtflow2_peer [format {{client-%s-%s}} [IP::client_addr] [TCP::client_port]]
        GENERICMESSAGE::peer name $evtflow2_peer
{event_log("CLIENT_ACCEPTED", "probe=mrf|peer=$evtflow2_peer")}
    }}
    when SERVER_CONNECTED {{
        set evtflow2_peer [format {{server-%s-%s}} [IP::server_addr] [TCP::server_port]]
        GENERICMESSAGE::peer name $evtflow2_peer
{event_log("SERVER_CONNECTED", "probe=mrf|peer=$evtflow2_peer")}
    }}
    when GENERICMESSAGE_INGRESS {{
        if {{ [clientside] }} {{
            GENERICMESSAGE::message is_request true
            GENERICMESSAGE::message dst evtflow-backend
        }}
        set evtflow2_mrf_sequence [GENERICMESSAGE::message request_sequence_number]
{event_log("GENERICMESSAGE_INGRESS", "probe=mrf|sequence=$evtflow2_mrf_sequence|length=[GENERICMESSAGE::message length]|request=[GENERICMESSAGE::message is_request]|src=[GENERICMESSAGE::message src]|dst=[GENERICMESSAGE::message dst]")}
    }}
    when MR_INGRESS {{
        set evtflow2_mrf_context ingress
        MR::store evtflow2_mrf_sequence
        MR::store evtflow2_mrf_context
{event_log("MR_INGRESS", "probe=mrf|sequence=$evtflow2_mrf_sequence|protocol=[MR::protocol]|flow=[MR::flow_id]|instance=[MR::instance]")}
    }}
    when MR_EGRESS {{
        MR::restore evtflow2_mrf_sequence
        MR::restore evtflow2_mrf_context
{event_log("MR_EGRESS", "probe=mrf|sequence=$evtflow2_mrf_sequence|restored=$evtflow2_mrf_context|protocol=[MR::protocol]|flow=[MR::flow_id]|route=[MR::message route]|status=[MR::message status]")}
    }}
    when GENERICMESSAGE_EGRESS {{
{event_log("GENERICMESSAGE_EGRESS", "probe=mrf|status=[GENERICMESSAGE::message status]|length=[GENERICMESSAGE::message length]")}
    }}
    when MR_FAILED {{
{event_log("MR_FAILED", "probe=mrf")}
    }}
}}
""",
    "irule",
)

write(
    "controls/mrf-collect-runtime.conf",
    f"""ltm rule {common}_mrf_collect_runtime {{
    when MR_INGRESS {{
{event_log("MR_INGRESS", "probe=mrf-collect-runtime")}
        MR::collect
    }}
    when MR_DATA {{
        binary scan [MR::payload] H* payload_hex
{event_log("MR_DATA", "probe=mrf-collect-runtime|bytes=[string length [MR::payload]]|hex=$payload_hex")}
        MR::release
    }}
}}
""",
    "runtime-control",
)


write(
    "controls/mr-failed-genericmessage-status.conf",
    f"""ltm rule {common}_mr_failed_genericmessage_status {{
    when MR_FAILED {{
{event_log("MR_FAILED", "probe=literal-control|status=[GENERICMESSAGE::message status]")}
    }}
}}
""",
    "literal-control",
)


def pool(name: str, offset: int) -> str:
    port = args.backend_port_base + offset
    return f"""ltm pool {common}_{name}_pool {{
    members {{
        {backend}:{port} {{ address {backend} }}
    }}
}}
"""


def virtual(name: str, offset: int, profiles: list[str], rules: list[str]) -> str:
    entries = "\n".join(f"        {profile} {{ }}" for profile in profiles)
    return f"""ltm virtual {common}_{name}_vs {{
    destination {vip}:{args.vip_port_base + offset}
    ip-protocol tcp
    mask 255.255.255.255
    pool {common}_{name}_pool
    profiles {{
{entries}
    }}
    rules {{ {" ".join(rules)} }}
    source 0.0.0.0/0
    source-address-translation {{ type automap }}
}}
"""


base = pool("tcp", 0) + virtual("tcp", 0, ["/Common/tcp"], [f"{common}_tcp_user_data"])
base += pool("ssl", 1) + virtual(
    "ssl",
    1,
    [f"{common}_clientssl", f"{common}_serverssl", "/Common/tcp"],
    [f"{common}_ssl_data"],
)
for offset, mode in ((9, "dynamic"), (10, "request"), (11, "require")):
    base += pool(f"client_cert_{mode}", offset) + virtual(
        f"client_cert_{mode}",
        offset,
        [f"{common}_clientssl_{mode}", "/Common/http", "/Common/tcp"],
        [f"{common}_client_cert"],
    )
base += pool("websocket", 2) + virtual(
    "websocket",
    2,
    ["/Common/http", "/Common/websocket", "/Common/tcp"],
    [f"{common}_websocket_data"],
)
base += pool("apm_allow", 3) + virtual(
    "apm_allow",
    3,
    [f"{common}_apm_allow", "/Common/http", "/Common/tcp"],
    [f"{common}_apm_trace"],
)
base += pool("apm_deny", 4) + virtual(
    "apm_deny",
    4,
    [f"{common}_apm_deny", "/Common/http", "/Common/tcp"],
    [f"{common}_apm_trace"],
)
base_without_asm = base
asm_virtuals = ""
for offset, mode in ((5, "transparent"), (6, "blocking")):
    asm_virtuals += pool(f"asm_{mode}", offset) + virtual(
        f"asm_{mode}",
        offset,
        ["/Common/http", "/Common/websecurity", "/Common/tcp"],
        [f"{common}_asm_{mode}_trace"],
    )
write(
    "config/20-ltm.conf",
    base_without_asm + asm_virtuals,
    "dependency-order-control",
)
write("config/20-ltm-base.conf", base_without_asm, "tmsh-config")
write("config/25-ltm-asm.conf", asm_virtuals, "tmsh-config")

ssl_profiles = f"""ltm profile client-ssl {common}_clientssl {{
    defaults-from /Common/clientssl
    cert-key-chain {{
        evtflow {{
            cert {common}_server
            key {common}_server
        }}
    }}
}}
ltm profile server-ssl {common}_serverssl {{
    defaults-from /Common/serverssl
    peer-cert-mode ignore
}}
"""
for mode, peer_mode in (
    ("dynamic", "ignore"),
    ("request", "request"),
    ("require", "require"),
):
    ssl_profiles += f"""ltm profile client-ssl {common}_clientssl_{mode} {{
    defaults-from /Common/clientssl
    authenticate always
    authenticate-depth 9
    ca-file {common}_ca
    cert-key-chain {{
        evtflow {{
            cert {common}_server
            key {common}_server
        }}
    }}
    client-cert-ca {common}_ca
    options {{ no-tlsv1.3 }}
    peer-cert-mode {peer_mode}
    retain-certificate true
}}
"""
write("config/15-ssl.conf", ssl_profiles, "tmsh-config")

apm = f"""apm policy agent ending-allow {common}_apm_allow_end_ag {{ }}
apm policy agent ending-deny {common}_apm_allow_deny_ag {{ }}
apm policy agent irule-event {common}_apm_agent_ag {{
    expect-data http
    id {args.run}_agent
}}
apm policy policy-item {common}_apm_allow_start {{
    caption Start
    rules {{ {{ caption fallback next-item {common}_apm_agent }} }}
}}
apm policy policy-item {common}_apm_agent {{
    agents {{ {common}_apm_agent_ag {{ type irule-event }} }}
    caption "iRule Event"
    color 1
    item-type action
    rules {{ {{ caption fallback next-item {common}_apm_allow_end }} }}
}}
apm policy policy-item {common}_apm_allow_end {{
    agents {{ {common}_apm_allow_end_ag {{ type ending-allow }} }}
    caption Allow
    color 1
    item-type ending
}}
apm policy policy-item {common}_apm_allow_deny {{
    agents {{ {common}_apm_allow_deny_ag {{ type ending-deny }} }}
    caption Deny
    color 2
    item-type ending
}}
apm policy access-policy {common}_apm_allow_policy {{
    default-ending {common}_apm_allow_deny
    items {{
        {common}_apm_allow_start {{ priority 1 }}
        {common}_apm_agent {{ priority 2 }}
        {common}_apm_allow_end {{ priority 3 }}
        {common}_apm_allow_deny {{ priority 4 }}
    }}
    start-item {common}_apm_allow_start
}}
apm profile access {common}_apm_allow {{
    defaults-from /Common/access
    access-policy {common}_apm_allow_policy
    generation-action increment
    max-failure-delay 0
    min-failure-delay 0
}}
apm policy agent ending-deny {common}_apm_deny_end_ag {{ }}
apm policy policy-item {common}_apm_deny_start {{
    caption Start
    rules {{ {{ caption fallback next-item {common}_apm_deny_end }} }}
}}
apm policy policy-item {common}_apm_deny_end {{
    agents {{ {common}_apm_deny_end_ag {{ type ending-deny }} }}
    caption Deny
    color 2
    item-type ending
}}
apm policy access-policy {common}_apm_deny_policy {{
    default-ending {common}_apm_deny_end
    items {{
        {common}_apm_deny_start {{ priority 1 }}
        {common}_apm_deny_end {{ priority 2 }}
    }}
    start-item {common}_apm_deny_start
}}
apm profile access {common}_apm_deny {{
    defaults-from /Common/access
    access-policy {common}_apm_deny_policy
    generation-action increment
    max-failure-delay 0
    min-failure-delay 0
}}
"""
write("config/10-apm.conf", apm, "tmsh-config")

mrf = (
    pool("mrf_flow", 7)
    + f"""ltm message-routing generic protocol {common}_mrf_protocol {{
    defaults-from /Common/genericmsg
    message-terminator %0A
    no-response no
}}
ltm message-routing generic transport-config {common}_mrf_transport {{
    profiles {{
        {common}_mrf_protocol {{ }}
        /Common/tcp {{ }}
    }}
    rules {{ {common}_mrf_trace }}
    source-address-translation {{ type automap }}
}}
ltm message-routing generic peer {common}_mrf_peer {{
    pool {common}_mrf_flow_pool
    transport-config {common}_mrf_transport
}}
ltm message-routing generic route {common}_mrf_route {{
    peers {{ {common}_mrf_peer }}
}}
ltm message-routing generic router {common}_mrf_router_flow {{
    defaults-from /Common/messagerouter
    irule-scope-message no
    routes {{ {common}_mrf_route }}
}}
ltm virtual {common}_mrf_flow_vs {{
    destination {vip}:{args.vip_port_base + 7}
    ip-protocol tcp
    mask 255.255.255.255
    profiles {{
        {common}_mrf_protocol {{ }}
        {common}_mrf_router_flow {{ }}
        /Common/tcp {{ }}
    }}
    rules {{ {common}_mrf_trace }}
    source 0.0.0.0/0
    source-address-translation {{ type automap }}
}}
ltm message-routing generic router {common}_mrf_router_message {{
    defaults-from /Common/messagerouter
    irule-scope-message yes
    routes {{ {common}_mrf_route }}
}}
ltm virtual {common}_mrf_message_vs {{
    destination {vip}:{args.vip_port_base + 8}
    ip-protocol tcp
    mask 255.255.255.255
    profiles {{
        {common}_mrf_protocol {{ }}
        {common}_mrf_router_message {{ }}
        /Common/tcp {{ }}
    }}
    rules {{ {common}_mrf_trace }}
    source 0.0.0.0/0
    source-address-translation {{ type automap }}
}}
"""
)
write("config/30-mrf.conf", mrf, "tmsh-config")

write(
    "load-asm.sh",
    f"""#!/bin/bash
set -u
for mode in transparent blocking; do
    policy={common}_asm_$mode
    blocking=disabled
    if [ "$mode" = blocking ]; then blocking=enabled; fi
    tmsh create asm policy "$policy" active blocking-mode "$blocking" encoding utf-8 policy-builder disabled policy-template POLICY_TEMPLATE_RAPID_DEPLOYMENT
    tmsh publish asm policy "$policy"
done
""",
    "appliance-script",
)

manifest_data = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode("ascii")
(args.out / "manifest.json").write_bytes(manifest_data)
lines = [f"{row['sha256']}  {row['file']}" for row in manifest["files"]]
lines.append(f"{sha256(manifest_data)}  manifest.json")
(args.out / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="ascii")

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

"""Generate byte-accounted iRule event load and traffic fixtures."""

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
from pathlib import Path


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def ascii_lf(text: str) -> bytes:
    data = text.encode("ascii")
    if b"\r" in data or b"\x00" in data:
        raise ValueError("generated fixture is not ASCII/LF-only")
    return data


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
if not 1024 <= args.vip_port_base <= 65400:
    parser.error("VIP port base must be 1024..65400")
if not 1024 <= args.backend_port_base <= 65400:
    parser.error("backend port base must be 1024..65400")

source_commit = args.source_commit
if source_commit is None:
    source_commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()

root = Path(__file__).resolve().parent
events = [
    line.strip()
    for line in (root / "event-candidates.txt").read_text(encoding="ascii").splitlines()
    if line.strip()
]
if events != sorted(set(events)):
    parser.error("event-candidates.txt must be sorted and unique")
if any(not re.fullmatch(r"[A-Z][A-Z0-9_]+", event) for event in events):
    parser.error("invalid event candidate")

args.out.mkdir(parents=True)
(args.out / "event-load").mkdir()
(args.out / "rules").mkdir()
(args.out / "config").mkdir()

prefix = f"__tcl_lsp_evtflow_{args.run}"
partition = f"EVTFLOW_{args.run}"
manifest = {
    "schema": 1,
    "run": args.run,
    "source_commit": source_commit,
    "line_endings": "LF",
    "encoding": "ASCII",
    "vip": str(vip),
    "backend": str(backend),
    "vip_port_base": args.vip_port_base,
    "backend_port_base": args.backend_port_base,
    "partition": partition,
    "prefix": prefix,
    "events": events,
    "files": [],
}


def write(relative: str, text: str, kind: str) -> None:
    data = ascii_lf(text)
    path = args.out / relative
    path.write_bytes(data)
    manifest["files"].append(
        {
            "file": relative,
            "kind": kind,
            "sha256": digest(data),
            "size": len(data),
            "ascii": data.isascii(),
            "contains_cr": b"\r" in data,
            "contains_nul": b"\x00" in data,
        }
    )


for event in events:
    rule = f"{prefix}_load_{event.lower()}"
    write(
        f"event-load/{event}.conf",
        f"""ltm rule /Common/{rule} {{
    when {event} priority 500 {{
        log local0. "EVTFLOW|run={args.run}|probe=event_load|event={event}|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
""",
        "event-load",
    )


def log_body(event: str, extra: str = "") -> str:
    fields = (
        f"EVTFLOW|run={args.run}|probe=flow|event={event}"
        "|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
        "|flow=$evtflow_flow|seq=$evtflow_seq"
    )
    if extra:
        fields += f"|{extra}"
    return f'        incr evtflow_seq\n        log local0. "{fields}"\n'


tcp_rule = [
    f"ltm rule /Common/{prefix}_tcp_trace {{",
    "    when RULE_INIT {",
    f'        log local0. "EVTFLOW|run={args.run}|probe=tcp|event=RULE_INIT|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"',
    "    }",
    "    when CLIENT_ACCEPTED {",
    "        set evtflow_seq 0",
    "        set evtflow_flow [format {%s:%s-%s:%s} [IP::client_addr] [TCP::client_port] [IP::local_addr] [TCP::local_port]]",
    log_body("CLIENT_ACCEPTED").rstrip(),
    "        TCP::collect",
    "    }",
    "    when CLIENT_DATA {",
    log_body("CLIENT_DATA", "bytes=[TCP::payload length]").rstrip(),
    "        TCP::release",
    "    }",
]
for event in ("LB_SELECTED", "SERVER_INIT", "SA_PICKED", "SERVER_CONNECTED"):
    tcp_rule.extend([f"    when {event} {{", log_body(event).rstrip()])
    if event == "SERVER_CONNECTED":
        tcp_rule.append("        TCP::collect")
    tcp_rule.append("    }")
tcp_rule.extend(
    [
        "    when LB_FAILED {",
        log_body("LB_FAILED").rstrip(),
        "    }",
        "    when LB_QUEUED {",
        log_body("LB_QUEUED").rstrip(),
        "    }",
        "    when SERVER_DATA {",
        log_body("SERVER_DATA", "bytes=[TCP::payload length]").rstrip(),
        "        TCP::release",
        "    }",
        "    when SERVER_CLOSED {",
        log_body("SERVER_CLOSED").rstrip(),
        "    }",
        "    when CLIENT_CLOSED {",
        log_body("CLIENT_CLOSED").rstrip(),
        "    }",
        "}",
        "",
    ]
)
write("rules/tcp-trace.conf", "\n".join(tcp_rule), "runtime-rule")

http_rule = f"""ltm rule /Common/{prefix}_http_trace {{
    when RULE_INIT {{
        log local0. "EVTFLOW|run={args.run}|probe=http|event=RULE_INIT|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CLIENT_ACCEPTED {{
        set evtflow_seq 0
        set evtflow_request none
        set evtflow_txn 0
        set evtflow_flow [format {{%s:%s-%s:%s}} [IP::client_addr] [TCP::client_port] [IP::local_addr] [TCP::local_port]]
{log_body("CLIENT_ACCEPTED").rstrip()}
    }}
    when HTTP_REQUEST {{
        incr evtflow_txn
        set evtflow_request [HTTP::header value X-Evtflow-Request]
        if {{$evtflow_request eq ""}} {{ set evtflow_request none }}
{log_body("HTTP_REQUEST", "request=$evtflow_request|txn=$evtflow_txn|method=[HTTP::method]|uri=[HTTP::uri]|version=[HTTP::version]").rstrip()}
        if {{[HTTP::header exists Content-Length] && [HTTP::header value Content-Length] > 0}} {{
            HTTP::collect [HTTP::header value Content-Length]
        }}
    }}
    when HTTP_REQUEST_DATA {{
{log_body("HTTP_REQUEST_DATA", "request=$evtflow_request|bytes=[HTTP::payload length]").rstrip()}
        HTTP::release
    }}
    when LB_SELECTED {{
{log_body("LB_SELECTED", "request=$evtflow_request|member=[LB::server addr]:[LB::server port]").rstrip()}
    }}
    when LB_FAILED {{
{log_body("LB_FAILED", "request=$evtflow_request").rstrip()}
    }}
    when SERVER_INIT {{
{log_body("SERVER_INIT", "request=$evtflow_request").rstrip()}
    }}
    when SA_PICKED {{
{log_body("SA_PICKED", "request=$evtflow_request").rstrip()}
    }}
    when SERVER_CONNECTED {{
{log_body("SERVER_CONNECTED", "request=$evtflow_request|peer=[IP::server_addr]:[TCP::server_port]").rstrip()}
    }}
    when HTTP_REQUEST_SEND {{
{log_body("HTTP_REQUEST_SEND", "request=$evtflow_request").rstrip()}
    }}
    when HTTP_REQUEST_RELEASE {{
{log_body("HTTP_REQUEST_RELEASE", "request=$evtflow_request").rstrip()}
    }}
    when HTTP_RESPONSE_CONTINUE {{
{log_body("HTTP_RESPONSE_CONTINUE", "request=$evtflow_request").rstrip()}
    }}
    when HTTP_RESPONSE {{
{log_body("HTTP_RESPONSE", "request=$evtflow_request|status=[HTTP::status]|version=[HTTP::version]").rstrip()}
        if {{[HTTP::header exists X-Evtflow-Collect]}} {{ HTTP::collect }}
    }}
    when HTTP_RESPONSE_DATA {{
{log_body("HTTP_RESPONSE_DATA", "request=$evtflow_request|bytes=[HTTP::payload length]").rstrip()}
        HTTP::release
    }}
    when HTTP_RESPONSE_RELEASE {{
{log_body("HTTP_RESPONSE_RELEASE", "request=$evtflow_request").rstrip()}
    }}
    when HTTP_DISABLED {{
{log_body("HTTP_DISABLED", "request=$evtflow_request").rstrip()}
    }}
    when HTTP_REJECT {{
{log_body("HTTP_REJECT", "request=$evtflow_request").rstrip()}
    }}
    when SERVER_CLOSED {{
{log_body("SERVER_CLOSED", "request=$evtflow_request").rstrip()}
    }}
    when CLIENT_CLOSED {{
{log_body("CLIENT_CLOSED", "request=$evtflow_request").rstrip()}
    }}
}}
"""
write("rules/http-trace.conf", http_rule, "runtime-rule")

clientssl_rule = f"""ltm rule /Common/{prefix}_clientssl_trace {{
    when CLIENT_ACCEPTED {{
        set evtflow_tls_seq 0
        set evtflow_tls_flow [format {{%s:%s-%s:%s}} [IP::client_addr] [TCP::client_port] [IP::local_addr] [TCP::local_port]]
    }}
    when CLIENTSSL_CLIENTHELLO {{
        incr evtflow_tls_seq
        log local0. "EVTFLOW|run={args.run}|probe=tls|event=CLIENTSSL_CLIENTHELLO|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|flow=$evtflow_tls_flow|seq=$evtflow_tls_seq"
    }}
    when CLIENTSSL_SERVERHELLO_SEND {{
        incr evtflow_tls_seq
        log local0. "EVTFLOW|run={args.run}|probe=tls|event=CLIENTSSL_SERVERHELLO_SEND|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|flow=$evtflow_tls_flow|seq=$evtflow_tls_seq"
    }}
    when CLIENTSSL_HANDSHAKE {{
        incr evtflow_tls_seq
        log local0. "EVTFLOW|run={args.run}|probe=tls|event=CLIENTSSL_HANDSHAKE|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|flow=$evtflow_tls_flow|seq=$evtflow_tls_seq|cipher=[SSL::cipher name]|version=[SSL::cipher version]"
    }}
}}
"""
write("rules/clientssl-trace.conf", clientssl_rule, "runtime-rule")

serverssl_rule = f"""ltm rule /Common/{prefix}_serverssl_trace {{
    when SERVERSSL_CLIENTHELLO_SEND {{
        log local0. "EVTFLOW|run={args.run}|probe=tls-server|event=SERVERSSL_CLIENTHELLO_SEND|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when SERVERSSL_SERVERHELLO {{
        log local0. "EVTFLOW|run={args.run}|probe=tls-server|event=SERVERSSL_SERVERHELLO|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when SERVERSSL_SERVERCERT {{
        log local0. "EVTFLOW|run={args.run}|probe=tls-server|event=SERVERSSL_SERVERCERT|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when SERVERSSL_HANDSHAKE {{
        log local0. "EVTFLOW|run={args.run}|probe=tls-server|event=SERVERSSL_HANDSHAKE|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|cipher=[SSL::cipher name]|version=[SSL::cipher version]"
    }}
}}
"""
write("rules/serverssl-trace.conf", serverssl_rule, "runtime-rule")

http3_rule = f"""ltm rule /Common/{prefix}_http3_trace {{
    when CLIENT_ACCEPTED {{
        log local0. "EVTFLOW|run={args.run}|probe=http3|event=CLIENT_ACCEPTED|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|client=[IP::client_addr]"
    }}
    when CLIENTSSL_CLIENTHELLO {{
        log local0. "EVTFLOW|run={args.run}|probe=http3|event=CLIENTSSL_CLIENTHELLO|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CLIENTSSL_HANDSHAKE {{
        log local0. "EVTFLOW|run={args.run}|probe=http3|event=CLIENTSSL_HANDSHAKE|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_REQUEST {{
        log local0. "EVTFLOW|run={args.run}|probe=http3|event=HTTP_REQUEST|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|version=[HTTP::version]|uri=[HTTP::uri]"
    }}
    when HTTP_RESPONSE {{
        log local0. "EVTFLOW|run={args.run}|probe=http3|event=HTTP_RESPONSE|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|version=[HTTP::version]|status=[HTTP::status]"
    }}
}}
"""
write("rules/http3-trace.conf", http3_rule, "runtime-rule")

udp_rule = f"""ltm rule /Common/{prefix}_udp_trace {{
    when RULE_INIT {{
        log local0. "EVTFLOW|run={args.run}|probe=udp|event=RULE_INIT|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CLIENT_ACCEPTED {{
        set evtflow_seq 0
        set evtflow_flow [format {{%s:%s-%s:%s}} [IP::client_addr] [UDP::client_port] [IP::local_addr] [UDP::local_port]]
{log_body("CLIENT_ACCEPTED", "bytes=[UDP::payload length]").rstrip()}
    }}
    when LB_SELECTED {{
{log_body("LB_SELECTED").rstrip()}
    }}
    when LB_FAILED {{
{log_body("LB_FAILED").rstrip()}
    }}
    when SERVER_CONNECTED {{
{log_body("SERVER_CONNECTED").rstrip()}
    }}
    when CLIENT_DATA {{
{log_body("CLIENT_DATA", "bytes=[UDP::payload length]").rstrip()}
    }}
    when SERVER_DATA {{
{log_body("SERVER_DATA", "bytes=[UDP::payload length]").rstrip()}
    }}
    when SERVER_CLOSED {{
{log_body("SERVER_CLOSED").rstrip()}
    }}
    when CLIENT_CLOSED {{
{log_body("CLIENT_CLOSED").rstrip()}
    }}
}}
"""
write("rules/udp-trace.conf", udp_rule, "runtime-rule")

dns_rule = f"""ltm rule /Common/{prefix}_dns_trace {{
    when CLIENT_ACCEPTED {{
        set evtflow_dns_seq 0
        set evtflow_dns_flow [format {{%s:%s-%s:%s}} [IP::client_addr] [UDP::client_port] [IP::local_addr] [UDP::local_port]]
    }}
    when DNS_REQUEST {{
        incr evtflow_dns_seq
        log local0. "EVTFLOW|run={args.run}|probe=dns|event=DNS_REQUEST|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|flow=$evtflow_dns_flow|seq=$evtflow_dns_seq|id=[DNS::header id]|name=[DNS::question name]|type=[DNS::question type]"
    }}
    when DNS_RESPONSE {{
        incr evtflow_dns_seq
        log local0. "EVTFLOW|run={args.run}|probe=dns|event=DNS_RESPONSE|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|flow=$evtflow_dns_flow|seq=$evtflow_dns_seq|id=[DNS::header id]|answers=[llength [DNS::answer]]"
    }}
}}
"""
write("rules/dns-trace.conf", dns_rule, "runtime-rule")

cache_rule = f"""ltm rule /Common/{prefix}_cache_trace {{
    when CLIENT_ACCEPTED {{
        set evtflow_cache_id none
        set evtflow_cache_seq 0
    }}
    when HTTP_REQUEST priority 50 {{
        set evtflow_cache_id [HTTP::header value X-Evtflow-Request]
    }}
    when CACHE_REQUEST {{
        incr evtflow_cache_seq
        log local0. "EVTFLOW|run={args.run}|probe=cache|event=CACHE_REQUEST|request=$evtflow_cache_id|seq=$evtflow_cache_seq|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CACHE_RESPONSE {{
        incr evtflow_cache_seq
        log local0. "EVTFLOW|run={args.run}|probe=cache|event=CACHE_RESPONSE|request=$evtflow_cache_id|seq=$evtflow_cache_seq|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CACHE_UPDATE {{
        incr evtflow_cache_seq
        log local0. "EVTFLOW|run={args.run}|probe=cache|event=CACHE_UPDATE|request=$evtflow_cache_id|seq=$evtflow_cache_seq|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
"""
write("rules/cache-trace.conf", cache_rule, "runtime-rule")

fastl4_rule = f"""ltm rule /Common/{prefix}_fastl4_trace {{
    when CLIENT_ACCEPTED {{
        log local0. "EVTFLOW|run={args.run}|probe=fastl4|event=CLIENT_ACCEPTED|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|client=[IP::client_addr]:[TCP::client_port]"
    }}
    when LB_SELECTED {{
        log local0. "EVTFLOW|run={args.run}|probe=fastl4|event=LB_SELECTED|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when SERVER_CONNECTED {{
        log local0. "EVTFLOW|run={args.run}|probe=fastl4|event=SERVER_CONNECTED|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CLIENT_CLOSED {{
        log local0. "EVTFLOW|run={args.run}|probe=fastl4|event=CLIENT_CLOSED|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when SERVER_CLOSED {{
        log local0. "EVTFLOW|run={args.run}|probe=fastl4|event=SERVER_CLOSED|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
"""
write("rules/fastl4-trace.conf", fastl4_rule, "runtime-rule")


def protocol_rule(name: str, events: tuple[str, ...]) -> None:
    lines = [f"ltm rule /Common/{prefix}_{name}_trace {{"]
    for event in events:
        lines.extend(
            [
                f"    when {event} {{",
                f'        log local0. "EVTFLOW|run={args.run}|probe={name}|event={event}|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"',
                "    }",
            ]
        )
    lines.extend(["}", ""])
    write(f"rules/{name}-trace.conf", "\n".join(lines), "runtime-rule")


protocol_rule(
    "sip",
    (
        "SIP_REQUEST",
        "SIP_REQUEST_SEND",
        "SIP_RESPONSE",
        "SIP_RESPONSE_SEND",
    ),
)

policy_rule = f"""ltm rule /Common/{prefix}_policy_trace {{
    when HTTP_REQUEST priority 100 {{
        log local0. "EVTFLOW|run={args.run}|probe=policy|event=HTTP_REQUEST|priority=100|header=[HTTP::header value X-Evtflow-Policy]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_REQUEST priority 900 {{
        log local0. "EVTFLOW|run={args.run}|probe=policy|event=HTTP_REQUEST|priority=900|header=[HTTP::header value X-Evtflow-Policy]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
"""
write("rules/policy-trace.conf", policy_rule, "runtime-rule")
protocol_rule(
    "rtsp",
    ("RTSP_REQUEST", "RTSP_REQUEST_DATA", "RTSP_RESPONSE", "RTSP_RESPONSE_DATA"),
)
protocol_rule(
    "mqtt",
    (
        "MQTT_CLIENT_INGRESS",
        "MQTT_CLIENT_DATA",
        "MQTT_CLIENT_EGRESS",
        "MQTT_SERVER_INGRESS",
        "MQTT_SERVER_DATA",
        "MQTT_SERVER_EGRESS",
        "MQTT_CLIENT_SHUTDOWN",
    ),
)

priority_rules = []
for priority in (100, 300, 500, 700, 900):
    rule_name = f"{prefix}_priority_{priority}"
    priority_rules.append(f"/{'Common'}/{rule_name}")
    write(
        f"rules/priority-{priority}.conf",
        f"""ltm rule /Common/{rule_name} {{
    when CLIENT_ACCEPTED priority {priority} {{
        log local0. "EVTFLOW|run={args.run}|probe=priority|event=CLIENT_ACCEPTED|priority={priority}|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_REQUEST priority {priority} {{
        log local0. "EVTFLOW|run={args.run}|probe=priority|event=HTTP_REQUEST|priority={priority}|request=[HTTP::header value X-Evtflow-Request]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_RESPONSE priority {priority} {{
        log local0. "EVTFLOW|run={args.run}|probe=priority|event=HTTP_RESPONSE|priority={priority}|status=[HTTP::status]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
""",
        "priority-rule",
    )

suppression = {
    "event-disable-response": "event disable HTTP_RESPONSE",
    "event-disable-request": "event disable HTTP_REQUEST",
    "event-disable-all": "event disable all",
    "http-disable-request": "HTTP::disable",
    "reject-only": "reject",
    "reject-disable-all": "reject\n        event disable all",
    "respond-only": "HTTP::respond 204 -version auto noserver Connection close",
    "respond-disable-all": "HTTP::respond 204 -version auto noserver Connection close\n        event disable all",
}
for name, action in suppression.items():
    indented = action.replace("\n", "\n        ")
    write(
        f"rules/{name}.conf",
        f"""ltm rule /Common/{prefix}_{name.replace("-", "_")} {{
    when HTTP_REQUEST priority 400 {{
        log local0. "EVTFLOW|run={args.run}|probe=suppression|case={name}|point=before|request=[HTTP::header value X-Evtflow-Request]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
        {indented}
        log local0. "EVTFLOW|run={args.run}|probe=suppression|case={name}|point=after|request=[HTTP::header value X-Evtflow-Request]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
""",
        "suppression-rule",
    )

safe_suppression = {
    "http-disable-safe": "HTTP::disable",
    "respond-safe": 'HTTP::respond 204 -version 1.1 -content "" Connection close -noserver',
    "respond-disable-all-safe": 'HTTP::respond 204 -version 1.1 -content "" Connection close -noserver\n        event disable all',
}
for name, action in safe_suppression.items():
    indented = action.replace("\n", "\n        ")
    write(
        f"rules/{name}.conf",
        f"""ltm rule /Common/{prefix}_{name.replace("-", "_")} {{
    when HTTP_REQUEST priority 400 {{
        set evtflow_suppression_id [HTTP::header value X-Evtflow-Request]
        log local0. "EVTFLOW|run={args.run}|probe=suppression-safe|case={name}|point=before|request=$evtflow_suppression_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
        {indented}
        log local0. "EVTFLOW|run={args.run}|probe=suppression-safe|case={name}|point=after|request=$evtflow_suppression_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
""",
        "suppression-rule",
    )

write(
    "rules/suppression-observer.conf",
    f"""ltm rule /Common/{prefix}_suppression_observer {{
    when HTTP_REQUEST priority 500 {{
        log local0. "EVTFLOW|run={args.run}|probe=suppression-observer|event=HTTP_REQUEST|request=$evtflow_suppression_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_DISABLED priority 500 {{
        log local0. "EVTFLOW|run={args.run}|probe=suppression-observer|event=HTTP_DISABLED|request=$evtflow_suppression_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_RESPONSE priority 500 {{
        log local0. "EVTFLOW|run={args.run}|probe=suppression-observer|event=HTTP_RESPONSE|request=$evtflow_suppression_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when CLIENT_CLOSED priority 500 {{
        log local0. "EVTFLOW|run={args.run}|probe=suppression-observer|event=CLIENT_CLOSED|request=$evtflow_suppression_id|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
""",
    "suppression-rule",
)

node = f"/{partition}/{prefix}_node"


def virtual_config(
    name: str,
    offset: int,
    protocol: str,
    profiles: str,
    rules: list[str],
    backend_offset: int | None = None,
) -> str:
    pool = f"/{partition}/{prefix}_{name}_pool"
    member_port = args.backend_port_base + (
        offset if backend_offset is None else backend_offset
    )
    return f"""ltm pool {pool} {{
    members {{
        {node}:{member_port} {{ address {backend} }}
    }}
}}
ltm virtual /{partition}/{prefix}_{name}_vs {{
    destination {vip}:{args.vip_port_base + offset}
    ip-protocol {protocol}
    mask 255.255.255.255
    pool {pool}
    profiles {{
{profiles}
    }}
    rules {{ {" ".join(rules)} }}
    source 0.0.0.0/0
    source-address-translation {{ type automap }}
}}
"""


write(
    "config/00-partition-node.conf",
    f"""auth partition {partition} {{
    default-route-domain 0
}}
ltm node {node} {{
    address {backend}
}}
""",
    "configuration",
)
write(
    "config/10-tcp.conf",
    virtual_config(
        "tcp",
        0,
        "tcp",
        "        /Common/tcp { }",
        [f"/Common/{prefix}_tcp_trace"],
    ),
    "configuration",
)
write(
    "config/11-http.conf",
    virtual_config(
        "http",
        1,
        "tcp",
        "        /Common/http { }\n        /Common/tcp { }",
        [f"/Common/{prefix}_http_trace", *priority_rules],
    ),
    "configuration",
)
write(
    "config/12-https.conf",
    virtual_config(
        "https",
        2,
        "tcp",
        "        /Common/clientssl { context clientside }\n        /Common/http { }\n        /Common/tcp { }",
        [
            f"/Common/{prefix}_http_trace",
            f"/Common/{prefix}_clientssl_trace",
            *priority_rules,
        ],
    ),
    "configuration",
)
write(
    "config/13-http2.conf",
    f"""ltm profile client-ssl /{partition}/{prefix}_http2_clientssl {{
    defaults-from /Common/clientssl
    renegotiation disabled
}}
"""
    + virtual_config(
        "http2",
        3,
        "tcp",
        f"        /{partition}/{prefix}_http2_clientssl {{ context clientside }}\n        /Common/http {{ }}\n        /Common/http2 {{ context clientside }}\n        /Common/tcp {{ }}",
        [
            f"/Common/{prefix}_http_trace",
            f"/Common/{prefix}_clientssl_trace",
            *priority_rules,
        ],
    ),
    "configuration",
)
write(
    "config/14-udp.conf",
    virtual_config(
        "udp",
        4,
        "udp",
        "        /Common/udp { }",
        [f"/Common/{prefix}_udp_trace"],
    ),
    "configuration",
)
write(
    "config/15-dns.conf",
    virtual_config(
        "dns",
        5,
        "udp",
        "        /Common/dns { }\n        /Common/udp { }",
        [f"/Common/{prefix}_udp_trace", f"/Common/{prefix}_dns_trace"],
    ),
    "configuration",
)
write(
    "config/16-cache.conf",
    f"""ltm profile web-acceleration /{partition}/{prefix}_cache_profile {{
    defaults-from /Common/webacceleration
    cache-object-min-size 0
}}
"""
    + virtual_config(
        "cache",
        6,
        "tcp",
        f"        /{partition}/{prefix}_cache_profile {{ }}\n        /Common/http {{ }}\n        /Common/tcp {{ }}",
        [f"/Common/{prefix}_http_trace", f"/Common/{prefix}_cache_trace"],
        backend_offset=1,
    ),
    "configuration",
)
write(
    "config/17-fastl4.conf",
    virtual_config(
        "fastl4",
        7,
        "tcp",
        "        /Common/fastL4 { }",
        [f"/Common/{prefix}_fastl4_trace"],
        backend_offset=0,
    ),
    "configuration",
)
write(
    "config/18-mptcp.conf",
    f"""ltm profile tcp /{partition}/{prefix}_mptcp_profile {{
    defaults-from /Common/tcp
    mptcp enabled
}}
"""
    + virtual_config(
        "mptcp",
        8,
        "tcp",
        f"        /{partition}/{prefix}_mptcp_profile {{ }}",
        [f"/Common/{prefix}_tcp_trace"],
        backend_offset=0,
    ),
    "configuration",
)
write(
    "config/19-sip.conf",
    virtual_config(
        "sip",
        9,
        "udp",
        "        /Common/sip { }\n        /Common/udp { }",
        [f"/Common/{prefix}_udp_trace", f"/Common/{prefix}_sip_trace"],
        backend_offset=4,
    ),
    "configuration",
)
write(
    "config/20-rtsp.conf",
    virtual_config(
        "rtsp",
        10,
        "tcp",
        "        /Common/rtsp { }\n        /Common/tcp { }",
        [f"/Common/{prefix}_tcp_trace", f"/Common/{prefix}_rtsp_trace"],
        backend_offset=0,
    ),
    "configuration",
)
write(
    "config/21-mqtt.conf",
    virtual_config(
        "mqtt",
        11,
        "tcp",
        "        /Common/mqtt { }\n        /Common/tcp { }",
        [f"/Common/{prefix}_tcp_trace", f"/Common/{prefix}_mqtt_trace"],
        backend_offset=0,
    ),
    "configuration",
)
write(
    "config/22-down.conf",
    virtual_config(
        "down",
        12,
        "tcp",
        "        /Common/http { }\n        /Common/tcp { }",
        [f"/Common/{prefix}_http_trace"],
        backend_offset=99,
    ),
    "configuration",
)
write(
    "config/23-policy.conf",
    f"""ltm policy /{partition}/{prefix}_policy {{
    requires {{ http }}
    strategy /Common/first-match
    rules {{
        insert_header {{
            ordinal 1
            conditions {{
                0 {{
                    http-uri
                    path
                    starts-with
                    values {{ /policy }}
                }}
            }}
            actions {{
                0 {{
                    http-header
                    insert
                    name X-Evtflow-Policy
                    value yes
                }}
            }}
        }}
    }}
}}
""",
    "configuration",
)
write(
    "config/24-serverssl.conf",
    virtual_config(
        "serverssl",
        14,
        "tcp",
        "        /Common/http { }\n        /Common/serverssl { context serverside }\n        /Common/tcp { }",
        [f"/Common/{prefix}_http_trace", f"/Common/{prefix}_serverssl_trace"],
    ),
    "configuration",
)
write(
    "config/25-http3.conf",
    f"""ltm profile client-ssl /{partition}/{prefix}_http3_clientssl {{
    defaults-from /Common/clientssl
    options {{ dont-insert-empty-fragments no-dtls no-ssl no-tlsv1 no-tlsv1.1 no-tlsv1.2 }}
    renegotiation disabled
}}
"""
    + virtual_config(
        "http3",
        15,
        "udp",
        f"        /{partition}/{prefix}_http3_clientssl {{ context clientside }}\n        /Common/http {{ }}\n        /Common/http3 {{ context clientside }}\n        /Common/httprouter {{ }}\n        /Common/quic {{ context clientside }}\n        /Common/udp {{ }}",
        [f"/Common/{prefix}_http3_trace"],
        backend_offset=3,
    ),
    "configuration",
)

manifest_data = ascii_lf(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
(args.out / "manifest.json").write_bytes(manifest_data)
hash_rows = [f"{entry['sha256']}  {entry['file']}" for entry in manifest["files"]]
hash_rows.append(f"{digest(manifest_data)}  manifest.json")
(args.out / "SHA256SUMS").write_bytes(ascii_lf("\n".join(sorted(hash_rows)) + "\n"))

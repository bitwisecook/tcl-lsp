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

"""Generate exact-source and event-adapted variable-lifecycle fixtures."""

import argparse
import hashlib
import json
import re
import shutil
from pathlib import Path


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def ascii_lf(text: str) -> bytes:
    data = text.encode("ascii")
    if b"\r" in data or b"\x00" in data:
        raise ValueError("generated source must be ASCII, NUL-free, and LF-only")
    return data


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--out", required=True, type=Path)
parser.add_argument("--source-commit", required=True)
parser.add_argument("--vip", default="192.168.9.24")
parser.add_argument("--vip-port", default=18861, type=int)
parser.add_argument("--global-vip-port", default=18862, type=int)
parser.add_argument("--backend", default="192.168.9.80")
parser.add_argument("--backend-port", default=18761, type=int)
args = parser.parse_args()
if args.out.exists():
    parser.error("output directory already exists")

here = Path(__file__).resolve().parent
repo = here.parents[3]
request = here / "VARIABLE_LIFECYCLE_CHECKS.md"
request_blocks = re.findall(rb"```tcl\n(.*?)```\n", request.read_bytes(), re.DOTALL)
if len(request_blocks) != 1:
    raise SystemExit(f"expected one request Tcl block, found {len(request_blocks)}")
request_body = request_blocks[0]
if (
    not request_body.endswith(b"\n")
    or not request_body.isascii()
    or b"\r" in request_body
):
    raise SystemExit("request block is not ASCII/LF with a trailing LF")

args.out.mkdir(parents=True)
originals = args.out / "originals"
adapted = args.out / "adapted"
sources = args.out / "sources"
for directory in (originals, adapted, sources):
    directory.mkdir()

run = "vl1"
prefix = f"__tcl_lsp_2286_{run}"
common = f"/Common/{prefix}"
main_body = request_body.replace(b"RUN_TOKEN", run.encode("ascii"))
(sources / "request-block.tcl").write_bytes(request_body)
(sources / "main-rule-body.tcl").write_bytes(main_body)
main_rule = ascii_lf(f"ltm rule {common}_main {{\n") + main_body + b"}\n"
(sources / "main.conf").write_bytes(main_rule)
event_marker = b"when HTTP_REQUEST {\n"
if main_body.count(event_marker) != 1 or not main_body.endswith(b"}\n"):
    raise SystemExit("cannot isolate the request event body")
procedure_source, event_source = main_body.split(event_marker, 1)
runtime_source = procedure_source + event_source[:-2]
(adapted / "main-runtime-script.tcl").write_bytes(runtime_source)
main_supported = ascii_lf(
    f"""ltm rule {common}_main_supported {{
    when HTTP_REQUEST {{
        if {{[HTTP::uri] eq "/r2286-{run}-backend"}} {{ return }}
        set payload [binary format H* {runtime_source.hex()}]
        set outer_rc [catch {{eval $payload}} outer_result]
        if {{$outer_rc != 0}} {{
            binary scan $outer_result H* outer_hex
            log local0.notice [list R2286VL {run} main_supported_outer tmm [TMM::cmp_group]:[TMM::cmp_unit] rc $outer_rc result_hex $outer_hex]
            HTTP::respond 500 content [list {run} $outer_rc $outer_hex] "Content-Type" "text/plain" "Connection" "close"
        }}
    }}
}}
"""
)
(sources / "main-supported.conf").write_bytes(main_supported)

data_root = repo / "rust/tcl-syntax/tests/data/native_generic_variable_consumers"
active_names = ["scalar", "element", "pending-read-chain", "recursive-read-write"]
trace_names = [
    "lset-recreate-receiver",
    "lappend-recreate-receiver",
    "quiet-missing-error-code",
    "quiet-read-callback-error",
    "sequential-output-link-rebind",
]
commands = [
    "append",
    "lappend",
    "empty_lappend",
    "lset",
    "scan",
    "lassign",
    "binary_scan",
    "catch_output",
    "failindex",
    "info_default",
]
name_kinds = ["plain", "qualified", "index", "binary_nul", "decomposed"]

cases: list[dict[str, str | int | bool]] = []


def retain_and_adapt(label: str, source: Path, *, event_trace: bool = False) -> bytes:
    original = source.read_bytes()
    if not original.isascii() or b"\r" in original or b"\x00" in original:
        raise SystemExit(f"unexpected source bytes in {source}")
    shutil.copyfile(source, originals / f"{label}.tcl")
    if original.count(b"puts $summary\n") != 1:
        raise SystemExit(f"expected one final puts in {source}")
    event = original.replace(b"puts $summary\n", b"")
    if event_trace:
        event = event.replace(
            b"lappend ::events ", b"upvar 1 events events; lappend events "
        )
    (adapted / f"{label}.tcl").write_bytes(event)
    cases.append(
        {
            "case": label,
            "source": str(source.relative_to(repo)),
            "source_sha256": digest(original),
            "adapted_sha256": digest(event),
            "event_trace_adaptation": event_trace,
        }
    )
    return event


event_payloads: list[tuple[str, bytes]] = []
for name in active_names:
    event_payloads.append(
        (
            f"active-{name}",
            retain_and_adapt(
                f"active-{name}",
                data_root / "active-unset" / f"{name}.tcl",
                event_trace=True,
            ),
        )
    )
for name in trace_names:
    event_payloads.append(
        (
            f"trace-{name}",
            retain_and_adapt(f"trace-{name}", data_root / f"trace-{name}.tcl"),
        )
    )
for command in commands:
    for name_kind in name_kinds:
        label = f"generic-{command}-{name_kind}"
        event_payloads.append(
            (
                label,
                retain_and_adapt(label, data_root / f"{command}-{name_kind}.tcl"),
            )
        )

custom_payloads = {
    "lappend-zero-recreate": b"""proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set v OLD; proc watch {n k op} {uplevel 1 {unset v; set v NEW}}; trace variable v r watch; set h lappend; set answer [$h v]; list $answer [set v]} r]
set summary [list $c [hex $r]]
set summary
""",
    "scan-rebind-frame-zero": b"""proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set replacement BEFORE; set first INIT; set second ORIGINAL; proc watch {n k op} {uplevel 1 {upvar 0 replacement second}}; trace variable first w watch; set n first; set m second; set h scan; set count [$h {7 9} {%d %d} $n $m]; list $count $first $second $replacement} r]
set summary [list $c [hex $r]]
set summary
""",
}
for label, payload in custom_payloads.items():
    (adapted / f"{label}.tcl").write_bytes(payload)
    event_payloads.append((label, payload))
    cases.append(
        {
            "case": label,
            "source": "generated event-frame control",
            "source_sha256": digest(payload),
            "adapted_sha256": digest(payload),
            "event_trace_adaptation": True,
        }
    )

control_lines = [
    "    when HTTP_REQUEST {\n",
    f'        if {{[HTTP::path] ne "/r2286-{run}-controls"}} {{ return }}\n',
    "        set rows {}\n",
]
for label, payload in event_payloads:
    control_lines.extend(
        [
            f"        set payload [binary format H* {payload.hex()}]\n",
            "        set rc [catch {eval $payload} result]\n",
            "        set scan_rc [catch {binary scan $result H* result_hex} scan_result]\n",
            f"        lappend rows [list {label} $rc $scan_rc $result_hex $scan_result]\n",
        ]
    )
control_lines.extend(
    [
        f"        log local0.notice [list R2286VL {run} controls tmm [TMM::cmp_group]:[TMM::cmp_unit] count [llength $rows] rows $rows]\n",
        f'        HTTP::respond 200 content [list {run} [TMM::cmp_group] [TMM::cmp_unit] $rows] "Content-Type" "text/plain" "Connection" "close"\n',
        "    }\n",
    ]
)
controls_rule = ascii_lf(
    f"ltm rule {common}_controls {{\n" + "".join(control_lines) + "}\n"
)
(sources / "controls.conf").write_bytes(controls_rule)

argument_payload = ascii_lf(
    f"""set events {{}}
set a(k) OLD
proc {prefix}_argument_watch {{n k op}} {{
    upvar 1 events events
    binary scan $n H* name_hex
    binary scan $k H* index_hex
    lappend events [list $op $name_hex $index_hex]
    if {{$op eq "read"}} {{uplevel 1 {{unset a(k); set a(k) NEW}}}}
}}
trace add variable a(k) {{read unset}} {prefix}_argument_watch
set h lappend
set answer [$h a(k) EXTRA]
set summary [list $answer [set a(k)] $events [trace info variable a(k)]]
set summary
"""
)
(adapted / "active-element-argument-bytes.tcl").write_bytes(argument_payload)
argument_rule = ascii_lf(
    f"""ltm rule {common}_argument_bytes {{
    when HTTP_REQUEST {{
        if {{[HTTP::path] ne "/r2286-{run}-argument-bytes"}} {{ return }}
        set payload [binary format H* {argument_payload.hex()}]
        set rc [catch {{eval $payload}} result]
        set scan_rc [catch {{binary scan $result H* result_hex}} scan_result]
        HTTP::respond 200 content [list {run} [TMM::cmp_group] [TMM::cmp_unit] $rc $scan_rc $result_hex $scan_result] "Content-Type" "text/plain" "Connection" "close"
    }}
}}
"""
)
(sources / "argument-bytes.conf").write_bytes(argument_rule)

global_payloads = {
    "quiet_missing": ascii_lf(
        "set ::errorCode SENTINEL; set h lappend; $h v; list [set ::errorCode] [set v]\n"
    ),
    "callback_error": ascii_lf(
        f"""set ::errorCode SENTINEL
set v OLD
proc {prefix}_error_watch {{n k op}} {{error BOOM}}
trace variable v r {prefix}_error_watch
set h lappend
set answer [$h v]
trace vdelete v r {prefix}_error_watch
list $answer [set ::errorCode] [set v]
"""
    ),
    "scan_rebind_global": ascii_lf(
        f"""set ::{prefix}_replacement BEFORE
set first INIT
set second ORIGINAL
proc {prefix}_global_watch {{n k op}} {{uplevel 1 {{upvar #0 ::{prefix}_replacement second}}}}
trace variable first w {prefix}_global_watch
set n first
set m second
set h scan
set count [$h {{7 9}} {{%d %d}} $n $m]
list $count $first $second [set ::{prefix}_replacement]
"""
    ),
}
global_lines = [
    "    when HTTP_REQUEST {\n",
    f'        if {{[HTTP::path] ne "/r2286-{run}-global"}} {{ return }}\n',
    "        set rows {}\n",
]
for label, payload in global_payloads.items():
    (adapted / f"{label}.tcl").write_bytes(payload)
    global_lines.extend(
        [
            f"        set payload [binary format H* {payload.hex()}]\n",
            "        set rc [catch {eval $payload} result]\n",
            "        set scan_rc [catch {binary scan $result H* result_hex} scan_result]\n",
            f"        lappend rows [list {label} $rc $scan_rc $result_hex $scan_result]\n",
        ]
    )
global_lines.extend(
    [
        f"        log local0.notice [list R2286VL {run} global tmm [TMM::cmp_group]:[TMM::cmp_unit] rows $rows]\n",
        f'        HTTP::respond 200 content [list {run} [TMM::cmp_group] [TMM::cmp_unit] $rows] "Content-Type" "text/plain" "Connection" "close"\n',
        "    }\n",
    ]
)
global_rule = ascii_lf(f"ltm rule {common}_global {{\n" + "".join(global_lines) + "}\n")
(sources / "global.conf").write_bytes(global_rule)

static_rule = ascii_lf(
    f"""ltm rule {common}_static {{
    when RULE_INIT {{
        set static::{prefix}_cell SEED
        log local0.notice "R2286VL|{run}|static|RULE_INIT|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|value=$static::{prefix}_cell"
    }}
    when CLIENT_ACCEPTED {{
        log local0.notice "R2286VL|{run}|static|CLIENT_ACCEPTED|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|session=[IP::client_addr]:[TCP::client_port]|value=$static::{prefix}_cell"
    }}
    when HTTP_REQUEST {{
        if {{![string match "/r2286-{run}-static/*" [HTTP::path]]}} {{ return }}
        set before $static::{prefix}_cell
        set action read
        if {{[HTTP::path] eq "/r2286-{run}-static/write"}} {{
            set static::{prefix}_cell [HTTP::query]
            set action write
        }} elseif {{[HTTP::path] eq "/r2286-{run}-static/unset"}} {{
            unset static::{prefix}_cell
            set missing [catch {{set static::{prefix}_cell}} missing_result]
            set static::{prefix}_cell RECREATED
            set action [list unset missing $missing missing_result $missing_result]
        }}
        set after $static::{prefix}_cell
        set row [list {run} [TMM::cmp_group] [TMM::cmp_unit] [IP::client_addr] [TCP::client_port] before $before action $action after $after]
        log local0.notice [list R2286VL static $row]
        HTTP::respond 200 content $row "Content-Type" "text/plain" "Connection" "close"
    }}
}}
"""
)
(sources / "static.conf").write_bytes(static_rule)

representative_payload = dict(event_payloads)["active-scalar"]
context_events_rule = ascii_lf(
    f"""ltm rule {common}_context_events {{
    when RULE_INIT {{
        set payload [binary format H* {representative_payload.hex()}]
        set rc [catch {{eval $payload}} result]
        set scan_rc [catch {{binary scan $result H* result_hex}} scan_result]
        log local0.notice [list R2286VL {run} context_event RULE_INIT tmm [TMM::cmp_group]:[TMM::cmp_unit] rc $rc scan_rc $scan_rc result_hex $result_hex scan_result $scan_result]
    }}
    when CLIENT_ACCEPTED {{
        set payload [binary format H* {representative_payload.hex()}]
        set rc [catch {{eval $payload}} result]
        set scan_rc [catch {{binary scan $result H* result_hex}} scan_result]
        log local0.notice [list R2286VL {run} context_event CLIENT_ACCEPTED tmm [TMM::cmp_group]:[TMM::cmp_unit] session [IP::client_addr]:[TCP::client_port] rc $rc scan_rc $scan_rc result_hex $result_hex scan_result $scan_result]
    }}
}}
"""
)
(sources / "context-events.conf").write_bytes(context_events_rule)

identity_rule = ascii_lf(
    f"""ltm rule {common}_identity {{
    when HTTP_REQUEST {{
        if {{[HTTP::path] eq "/r2286-{run}-identity"}} {{
            HTTP::respond 200 content [list {run} [TMM::cmp_group] [TMM::cmp_unit] [IP::client_addr] [TCP::client_port]] "Content-Type" "text/plain" "Connection" "close"
        }}
    }}
}}
"""
)
(sources / "identity.conf").write_bytes(identity_rule)

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
ltm virtual {common}_global_vs {{
    destination {args.vip}:{args.global_vip_port}
    mask 255.255.255.255
    ip-protocol tcp
    source 0.0.0.0/0
    profiles {{ /Common/tcp {{ }} /Common/http {{ }} }}
    pool {common}_pool
    rules {{ {common}_global }}
    source-address-translation {{ type automap }}
    cmp-enabled yes
}}
"""
)
(sources / "lab.conf").write_bytes(lab)

manifest = {
    "run": run,
    "source_commit": args.source_commit,
    "request_sha256": digest(request.read_bytes()),
    "request_block_sha256": digest(request_body),
    "main_body_sha256": digest(main_body),
    "vip": f"{args.vip}:{args.vip_port}",
    "global_vip": f"{args.vip}:{args.global_vip_port}",
    "backend": f"{args.backend}:{args.backend_port}",
    "cases": cases,
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

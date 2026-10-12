# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Prepare bounded native visibility and worker/connection persistence controls.

Preparation is local. Only --execute deploys the retained configuration. Runtime
names are owned by an absence guard and a run marker; cleanup runs on RULE_INIT
units. Activated native-catalog retirement is not claimed by config deletion.
"""

import argparse
import datetime
import http.client
import json
import re
import shlex
import shutil
from pathlib import Path

import capture
from parse_capture import parse_capture

CONTEXTS = ["tmm-RULE_INIT", "tmm-procedure", "tmm-traffic-event"]


def build_plan(tag):
    if not re.fullmatch(r"[A-Za-z0-9_]+", tag):
        raise ValueError(
            "Run tag must contain only ASCII letters, digits and underscores"
        )
    name = "__tcl_lsp_probe_" + tag
    provider = "/Common/" + name + "_provider"
    caller = "/Common/" + name
    ns = "::__nh_" + tag
    regular = "::__nh_" + tag + "_global"
    static = "static::__nh_" + tag + "_static"
    marker = "static::__nh_" + tag + "_owner"
    connection = "__nh_" + tag + "_connection"
    dynamic = "__nh_" + tag + "_dynamic"
    tail = "__nh_" + tag + "_target"
    target = provider + "::" + tail
    moved = provider + "::" + tail + "_moved"
    state_guard = f"""set n namespace
if {{[$n exists {ns}]}} {{error {{Owned namespace collision}}}}
foreach cell [list {regular} {static} {marker}] {{
 if {{[info exists $cell] || [llength [info vars $cell]] || [llength [trace vinfo $cell]]}} {{error [list Owned-cell-collision $cell]}}
}}
foreach owned [list ::{dynamic}] {{if {{[llength [info commands $owned]]}} {{error [list Owned-dynamic-collision $owned]}}}}
foreach head [list {tail} {target} {moved}] {{if {{[llength [info commands $head]]}} {{error [list Owned-command-collision $head]}}}}
list absent {ns} {regular} {static} {marker} {tail} {target} {moved}
"""
    seed = f"""{state_guard.split("foreach head", 1)[0]}
set {marker} {tag}
set n namespace
$n eval {ns} {{}}
set {ns}::cell SEED
proc {ns}::{dynamic} {{}} {{return DYNAMIC}}
set {regular} SEED
set {static} SEED
list seeded {tag}
"""
    read = f"""set n namespace
set rows {{}}
foreach cell [list {ns}::cell {regular} {static}] {{set c [catch {{set $cell}} v]; lappend rows $cell $c $v}}
set c [catch {{eval [list {ns}::{dynamic}]}} v]
list cells $rows dynamic $c $v namespace [$n current] level [info level]
"""
    worker_action = f"""set action [HTTP::header value X-NH-Action]
if {{$action == "worker-write"}} {{foreach cell [list {ns}::cell {regular} {static}] {{set $cell WRITE}}}}
if {{$action == "worker-unset"}} {{foreach cell [list {ns}::cell {regular} {static}] {{unset -nocomplain $cell}}}}
if {{$action == "worker-recreate"}} {{foreach cell [list {ns}::cell {regular} {static}] {{set $cell RECREATE}}}}
{read}
"""
    visibility = f"""set n namespace
set before [list [info commands {tail}] [info procs {tail}] [info commands {target}] [info procs {target}] [$n which -command {tail}] [$n which -command {target}]]
set c [catch {{call {target} CATALOG}} v]
list catalogue $before native-call $c $v
"""
    ordinary = f"set head {tail}; $head ORDINARY"
    rename = f"""set before [catch {{call {target} BEFORE}} bv]
set r rename
set rc [catch {{$r {target} {moved}}} rv]
set old [catch {{call {target} OLD}} ov]
set new [catch {{call {moved} NEW}} nv]
set restore not-needed
set restored {{}}
if {{$rc == 0}} {{set restore [catch {{$r {moved} {target}}} restored]}}
list before $before $bv rename $rc $rv old $old $ov new $new $nv restore $restore $restored
"""
    cleanup = f"""set owner 0
if {{[info exists {marker}] && [set {marker}] == "{tag}"}} {{set owner 1}}
if {{$owner}} {{
 set n namespace
 if {{[$n exists {ns}]}} {{$n delete {ns}}}
 set r rename
 if {{[llength [info commands ::{dynamic}]]}} {{$r ::{dynamic} {{}}}}
 foreach cell [list {regular} {static} {marker}] {{unset -nocomplain $cell}}
}}
set n namespace
list owner $owner namespace [$n exists {ns}] regular [info exists {regular}] static [info exists {static}] marker [info exists {marker}] fallback-dynamic [info commands ::{dynamic}] namespace-dynamic [info commands {ns}::{dynamic}]
"""

    def case(case_id, question, operation, setup=""):
        return {
            "id": case_id,
            "question_id": question,
            "variant_id": "native-persistence-bounded-v1",
            "setup": setup,
            "operation": operation,
            "scope": "Exact newly authored owned control; native F5 provider and actual host frame. Nested completion codes/results are Tcl list data; unavailable inner error options are not inferred.",
            "adaptation": "New bounded materialisation; not an original portable corpus answer.",
        }

    native = [
        case("native-computed-ordinary-head", "F5-N11", ordinary),
        case("native-catalogue-versus-call", "F5-N11", visibility),
        case("native-computed-rename", "F5-N11", rename),
    ]
    worker = case("worker-cells-read", "F5-N13", read)
    connection_case = case(
        "connection-cells-read-write",
        "F5-N13",
        f"""set action [HTTP::header value X-NH-Action]
if {{$action == "connection-write"}} {{set {connection} [list WRITE [HTTP::header value X-NH-Invocation]]}}
list connection [set {connection}] peer [IP::client_addr] [TCP::client_port] level [info level]
""",
    )
    seed_case = case("worker-RULE_INIT-seed", "F5-N13", seed)
    preflight = case("worker-owned-absence", "F5-N13", state_guard)
    cleanup_case = case("worker-owned-cleanup", "F5-N13", cleanup)
    metadata = {
        "tmm-RULE_INIT": [preflight, seed_case, *native, worker, cleanup_case],
        "tmm-procedure": [*native, worker],
        "tmm-traffic-event": [
            *native,
            case("worker-cells-read", "F5-N13", worker_action),
            connection_case,
        ],
    }
    for item in metadata["tmm-procedure"]:
        item["invocation_events"] = {"init-procedure": "RULE_INIT", "*": "HTTP_REQUEST"}
    recorder = capture.emit(
        "tmm-RECORD", tag, "$nh_case", "$nh_stage", "$nh_code", "$nh_value", "$nh_unit"
    ).replace("|tmm-RECORD|", "|$nh_context|")
    record_proc = (
        "proc record {nh_context nh_case nh_stage nh_code nh_value nh_invocation} {\nset nh_options_available unavailable\nset nh_return_options {}\nset nh_unit [TMM::cmp_group]:[TMM::cmp_unit]\n"
        + recorder
        + "\n}\n"
    )

    def body(items, context, invocation):
        result = "set nh_invocation " + invocation + "\n"
        for item in items:
            result += f"# NHO|{tag}|{item['id']}|{context}|\n"
            result += (
                "set nh_setup "
                + capture.literal(item["setup"])
                + "\nset nh_operation "
                + capture.literal(item["operation"])
                + "\n"
            )
            result += "set nh_setup_code [catch {eval $nh_setup} nh_setup_result]\n"
            result += f"call {provider}::record {context} {item['id']} setup $nh_setup_code $nh_setup_result $nh_invocation\n"
            result += "if {$nh_setup_code == 0} {set nh_code [catch {eval $nh_operation} nh_value]\n"
            result += f"call {provider}::record {context} {item['id']} lookup $nh_code $nh_value $nh_invocation\n}}\n"
        return result

    procedure = body([*native, worker], "tmm-procedure", "$invocation")
    init = body([seed_case], "tmm-RULE_INIT", "init")
    # No operations may run after a failed seed/ownership guard.
    init += (
        "if {$nh_code == 0} {\n"
        + body([*native, worker], "tmm-RULE_INIT", "init")
        + f"call {provider}::measure init-procedure\n}}\n"
    )
    traffic = f'if {{![info exists {marker}] || [set {marker}] != "{tag}"}} {{HTTP::respond 503 content {{UNOWNED}} Connection close; return}}\n'
    traffic += body(
        metadata["tmm-traffic-event"],
        "tmm-traffic-event",
        "[HTTP::header value X-NH-Invocation]",
    )
    traffic += f'call {provider}::measure $nh_invocation\nHTTP::respond 200 content "{name} [TMM::cmp_group]:[TMM::cmp_unit]" X-NH-TMM "[TMM::cmp_group]:[TMM::cmp_unit]" X-NH-Invocation $nh_invocation Connection keep-alive\n'
    accepted = f"if {{[info exists {connection}]}} {{error {{Owned connection-cell collision}}}}\nset {connection} [list SEED {tag} [IP::client_addr] [TCP::client_port]]\n"
    config = f"ltm rule {provider} {{\nproc {tail} {{value}} {{list NATIVE $value}}\n{record_proc}proc measure {{invocation}} {{\n{procedure}}}\n}}\nltm rule {caller} {{\nwhen RULE_INIT {{\n{init}}}\nwhen CLIENT_ACCEPTED {{\n{accepted}}}\nwhen HTTP_REQUEST {{\n{traffic}}}\n}}\nltm virtual {caller}_vs {{ destination 192.168.9.24:19912 mask 255.255.255.255 ip-protocol tcp profiles {{ tcp {{}} http {{}} }} rules {{ {provider} {caller} }} }}\n"
    source = record_proc + procedure + init + accepted + traffic
    return {
        "tag": tag,
        "name": name,
        "provider": provider,
        "caller": caller,
        "config": config,
        "source": source,
        "objects": [
            ("ltm rule", provider),
            ("ltm rule", caller),
            ("ltm virtual", caller + "_vs"),
        ],
        "metadata": metadata,
        "preflight": preflight,
        "cleanup": cleanup_case,
        "limits": {
            "initial_expected_tmm_units": 4,
            "fresh_requests": 16,
            "keepalive_candidates": 5,
            "state_resets_in_HTTP_REQUEST": False,
            "native_catalog_retirement": "unavailable; config deletion only",
            "cleanup_scope": "Exact owned dynamic namespace/regular global/static cells on every RULE_INIT unit; connection cells expire with owned TCP connections.",
        },
    }


def paired_requests(dest, _remote, _context, name, index):
    if index != 8:
        return
    connections = []
    observations = []

    def request(conn, invocation, action="read"):
        conn.request(
            "GET", "/", headers={"X-NH-Invocation": invocation, "X-NH-Action": action}
        )
        response = conn.getresponse()
        body = response.read()
        row = {
            "invocation": invocation,
            "action": action,
            "status": response.status,
            "headers": response.getheaders(),
            "body_hex": body.hex(),
            "run_marker_matches": name.encode() in body,
            "unit": response.getheader("X-NH-TMM"),
        }
        observations.append(row)
        if response.status != 200 or not row["run_marker_matches"] or not row["unit"]:
            raise RuntimeError("Owned keepalive response not proved")
        return row["unit"]

    try:
        by_unit = {}
        pair = None
        for i in range(5):
            conn = http.client.HTTPConnection("192.168.9.24", 19912, timeout=3)
            connections.append(conn)
            unit = request(conn, "keep-candidate-" + str(i))
            if unit in by_unit:
                pair = (by_unit[unit], conn)
                break
            by_unit[unit] = conn
        if pair is None:
            observations.append(
                {
                    "status": "unavailable",
                    "reason": "No two bounded candidate TCP connections observed on the same actual TMM unit",
                }
            )
        else:
            a, b = pair
            for conn, invocation, action in [
                (a, "keep-a-write", "connection-write"),
                (b, "keep-b-read", "read"),
                (a, "keep-a-read", "read"),
                (a, "keep-worker-write", "worker-write"),
                (b, "keep-worker-written", "read"),
                (a, "keep-worker-unset", "worker-unset"),
                (b, "keep-worker-missing", "read"),
                (a, "keep-worker-recreate", "worker-recreate"),
                (b, "keep-worker-recreated", "read"),
            ]:
                request(conn, invocation, action)
    except Exception as error:
        observations.append(
            {
                "status": "infrastructure-failure",
                "error_type": type(error).__name__,
                "error": str(error),
            }
        )
    finally:
        for conn in connections:
            conn.close()
        (dest / "paired-keepalive.json").write_text(
            json.dumps(observations, indent=2) + "\n"
        )


def execute(plan, dest):
    tag = plan["tag"]
    capture.TAG = tag
    remote = "/var/tmp/tcl_lsp_" + tag
    if capture.rpc(
        "test ! -e " + shlex.quote(remote) + " && mkdir -m 700 " + shlex.quote(remote)
    ).returncode:
        raise RuntimeError("Remote staging collision")
    for label, command in [
        ("version", "tmsh show sys version"),
        ("platform", "tmsh show sys hardware"),
        ("tmm-roster", "tmsh show sys tmm-info"),
    ]:
        r = capture.rpc(command)
        (dest / (label + ".stdout")).write_bytes(r.stdout)
        (dest / (label + ".stderr")).write_bytes(r.stderr)
        (dest / (label + ".exit")).write_text(str(r.returncode) + "\n")
    preflight_path = dest / "tmm-RULE_INIT-0"
    capture.run_job(
        preflight_path,
        remote,
        "tmm-RULE_INIT",
        plan["name"] + "_preflight",
        capture.body([plan["preflight"]], "tmm-RULE_INIT", tag),
        wait=5,
    )
    parsed = parse_capture(
        [preflight_path / "runtime-logs.stdout"], "tmm-RULE_INIT", tag
    )
    # Raw parser entries are checked without assuming absent rows succeeded.
    rows = [
        row
        for row in parsed["fields"]
        if row.get("case_id") == plan["preflight"]["id"]
        and row.get("stage") == "lookup"
        and row.get("field") == "result"
        and row.get("complete")
    ]
    if len(
        {row.get("tmm_unit") for row in rows if row.get("completion_code") == 0}
    ) != 4 or any(row.get("completion_code") != 0 for row in rows):
        raise RuntimeError(
            "All four actual preflight units did not prove absence; no main configuration loaded"
        )
    original_config = capture.make_config
    original_hook = capture.TRAFFIC_HOOK
    job = dest / "tmm-traffic-event-0"
    try:
        capture.make_config = lambda _c, _n, _s, port=19912: (
            plan["config"],
            plan["objects"],
            None,
        )
        capture.TRAFFIC_HOOK = paired_requests
        capture.run_job(
            job, remote, "tmm-traffic-event", plan["name"], plan["source"], wait=5
        )
    finally:
        capture.make_config = original_config
        capture.TRAFFIC_HOOK = original_hook
        capture.run_job(
            dest / "tmm-RULE_INIT-2",
            remote,
            "tmm-RULE_INIT",
            plan["name"] + "_cleanup",
            capture.body([plan["cleanup"]], "tmm-RULE_INIT", tag),
            wait=5,
        )
    for context, number in [("tmm-RULE_INIT", 1), ("tmm-procedure", 0)]:
        alias = dest / (context + "-" + str(number))
        shutil.copytree(job, alias)
        (alias / "evidence-alias.json").write_text(
            json.dumps(
                {
                    "source_job": "tmm-traffic-event-0",
                    "context_filter": context,
                    "independent_execution": False,
                },
                indent=2,
            )
            + "\n"
        )
    r = capture.rpc("grep -F " + shlex.quote("NHO|" + tag + "|") + " /var/log/ltm")
    (dest / "final-runtime.stdout").write_bytes(r.stdout)
    (dest / "final-runtime.stderr").write_bytes(r.stderr)
    (dest / "final-runtime.exit").write_text(str(r.returncode) + "\n")
    (dest / "capture-complete.json").write_text(
        json.dumps({"completed": True, "configuration_saved": False}) + "\n"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", required=True)
    parser.add_argument("--execute", action="store_true")
    parser.add_argument("--destination", type=Path)
    args = parser.parse_args()
    plan = build_plan(args.run)
    dest = (
        args.destination
        or capture.ROOT / "experiments/name-handling/f5/answers" / args.run
    )
    dest.mkdir(parents=True, exist_ok=False)
    for path in [Path(__file__), Path(capture.__file__)]:
        (dest / path.name).write_bytes(path.read_bytes())
    (dest / "prepared.conf").write_text(plan["config"])
    (dest / "plan.json").write_text(json.dumps(plan, indent=2) + "\n")
    for context, items in plan["metadata"].items():
        (dest / (context + "-cases.json")).write_text(
            json.dumps(items, indent=2) + "\n"
        )
    (dest / "provenance.json").write_text(
        json.dumps(
            {
                "run_id": args.run,
                "timestamp_utc": datetime.datetime.now(
                    datetime.timezone.utc
                ).isoformat(),
                "configuration_saved": False,
                "remote_staging": "/var/tmp/tcl_lsp_" + args.run,
                "execution_requested": args.execute,
                "prepared_config_sha256": capture.digest(plan["config"].encode()),
                "limits": plan["limits"],
            },
            indent=2,
        )
        + "\n"
    )
    if args.execute:
        execute(plan, dest)
    print(dest)


if __name__ == "__main__":
    main()

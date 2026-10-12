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

"""Prepare native question controls; only --execute contacts the appliance."""

import argparse
import datetime
import http.client
import json
import re
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / "scripts/dev/bigip-probes/name-patterns"))
import capture
from parse_capture import parse_capture

PHASES = ["old", "new", "rejected", "detached", "deleted", "recreated"]
QUESTIONS = {
    "corpus-f5-rule-path": "For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?",
    "corpus-f5-computed-target": "Does a variable/list-built call target with exactly the literal-control bytes reach the same rule procedure?",
    "corpus-f5-prefix-arguments": "How does call pass fixed/list-expanded arguments containing spaces, braces and namespace separators?",
    "corpus-f5-rule-lifetime": "Does binding change at load, attachment, or each invocation when an isolated provider rule is replaced or detached?",
    "corpus-f5-storage-context": "Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?",
}


def build_plan(tag):
    if not re.fullmatch(r"[a-z][a-z0-9]{5,23}", tag):
        raise ValueError("Run tag must be 6–24 lowercase ASCII letters/digits")
    stem = "__tcl_lsp_probe_" + tag
    folder = "/Common/" + stem + "_folder"
    partition = "NH_" + tag
    provider = "/Common/" + stem + "_provider"
    other = "/Common/" + stem + "_other"
    folded = folder + "/" + stem + "_folded"
    foreign = "/" + partition + "/" + stem + "_foreign"
    caller = folder + "/" + stem + "_caller"
    tail = "__nh_" + tag + "_target"
    local = "__nh_" + tag + "_local"
    conn = "__nh_" + tag + "_connection"
    ns = "::__nh_" + tag
    cells = [
        ns + "::cell",
        "::__nh_" + tag + "_global",
        "static::__nh_" + tag + "_static",
    ]
    marker = "static::__nh_" + tag + "_owner"
    routes = [provider, other, folded, foreign]

    def case(case_id, qid, op):
        return {
            "id": case_id,
            "question_id": qid,
            "canonical_question": QUESTIONS[qid],
            "setup": "",
            "operation": op,
            "variant_id": "native-owned-completion-v1",
            "scope": "New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.",
        }

    cases = []
    paths = [
        ("bare-local", tail),
        ("root-relative", stem + "_provider::" + tail),
        ("root-absolute", provider + "::" + tail),
        ("other-owner", other + "::" + tail),
        ("folder-absolute", folded + "::" + tail),
        ("folder-leaf-relative", stem + "_folded::" + tail),
        ("partition-absolute", foreign + "::" + tail),
        ("missing-rule", "/Common/" + stem + "_absent::" + tail),
        ("missing-proc", provider + "::__nh_" + tag + "_absent"),
    ]
    for label, target in paths:
        cases.append(
            case(label, "corpus-f5-rule-path", "call " + target + " {a b} {{x} ::tail}")
        )
    target = provider + "::" + tail
    cases += [
        case(
            "computed-variable-target",
            "corpus-f5-computed-target",
            "set target "
            + target
            + "; binary scan $target H* target_hex; list target-hex $target_hex value [eval [list call $target {a b} {{x} ::tail}]]",
        ),
        case(
            "computed-list-target",
            "corpus-f5-computed-target",
            "set target [join [list "
            + provider
            + " "
            + tail
            + "] ::]; binary scan $target H* target_hex; list target-hex $target_hex value [eval [list call $target {a b} {{x} ::tail}]]",
        ),
        case(
            "literal-matched-arguments",
            "corpus-f5-prefix-arguments",
            "call " + target + " {a b} {{x} ::tail}",
        ),
        case(
            "list-prefix-arguments",
            "corpus-f5-prefix-arguments",
            "set prefix [list call "
            + target
            + " {a b}]; eval [linsert $prefix end {{x} ::tail}]",
        ),
        case(
            "space-joined-near-miss",
            "corpus-f5-prefix-arguments",
            'set prefix "call ' + target + ' a b"; eval "$prefix c d"',
        ),
        case(
            "provider-lifetime",
            "corpus-f5-rule-lifetime",
            "list phase $nh_phase value [call " + target + " {a b} {{x} ::tail}]",
        ),
        case(
            "provider-local-upvar-write",
            "corpus-f5-storage-context",
            "set "
            + local
            + " CALLER_BEFORE; set v [call "
            + provider
            + "::link "
            + local
            + " $nh_invocation]; list provider $v caller [set "
            + local
            + "]",
        ),
        case(
            "provider-shared-write-readback",
            "corpus-f5-storage-context",
            "set provider_value [call "
            + provider
            + "::cells $nh_action $nh_invocation]; set rows {}; foreach cell [list "
            + " ".join(cells)
            + "] {set c [catch {set $cell} v]; lappend rows $cell $c $v}; list phase $nh_phase action $nh_action provider $provider_value caller $rows",
        ),
    ]
    connection_case = case(
        "connection-identity-write-read",
        "corpus-f5-storage-context",
        "if {$nh_action == {connection-write}} {set "
        + conn
        + " [list CONNECTION_WRITE $nh_invocation]}; list cell [set "
        + conn
        + "] peer [IP::client_addr] [TCP::client_port] unit [TMM::cmp_group]:[TMM::cmp_unit]",
    )
    targets = [path + "::" + tail for path in routes]
    guard = f"""set n namespace
if {{[$n exists {ns}]}} {{error {{Owned namespace collision}}}}
foreach cell [list {" ".join(cells[1:] + [marker])}] {{if {{[info exists $cell] || [llength [trace vinfo $cell]]}} {{error [list Owned-cell-collision $cell]}}}}
foreach target [list {" ".join(targets)}] {{
 set c [catch {{eval [list call $target __guard__ __guard__]}} v]
 if {{$c != 1 || ![string match {{*not found*}} $v]}} {{error [list Native-absence-not-proved $target $c $v]}}
}}
list absent {ns} {" ".join(cells[1:] + [marker])} {" ".join(targets)}
"""
    seed = f"""set n namespace
if {{[$n exists {ns}] || [info exists {marker}]}} {{error {{Owned state seed collision}}}}
foreach cell [list {" ".join(cells[1:])}] {{if {{[info exists $cell]}} {{error [list Owned-cell-collision $cell]}}}}
$n eval {ns} {{}}
set {marker} {tag}
set {cells[0]} NAMESPACE_SEED
set {cells[1]} GLOBAL_SEED
set {cells[2]} STATIC_SEED
list seeded {tag}
"""
    cleanup = f"""set n namespace
set owner 0
if {{[info exists {marker}] && [set {marker}] == "{tag}"}} {{set owner 1}}
if {{$owner}} {{if {{[$n exists {ns}]}} {{$n delete {ns}}}; foreach cell [list {" ".join(cells[1:] + [marker])}] {{unset -nocomplain $cell}}}}
list owner $owner namespace [$n exists {ns}] regular [info exists {cells[1]}] static [info exists {cells[2]}] marker [info exists {marker}]
"""
    preflight = case("owned-native-state-absence", "corpus-f5-storage-context", guard)
    seed_case = case("owned-state-seed", "corpus-f5-storage-context", seed)
    cleanup_case = case("owned-state-cleanup", "corpus-f5-storage-context", cleanup)
    metadata = {
        "tmm-RULE_INIT": [
            preflight,
            seed_case,
            *[dict(c) for c in cases],
            cleanup_case,
        ],
        "tmm-procedure": [dict(c) for c in cases],
        "tmm-traffic-event": [*[dict(c) for c in cases], connection_case],
    }
    for item in metadata["tmm-procedure"]:
        item["invocation_events"] = {
            **{phase + "-init-procedure": "RULE_INIT" for phase in PHASES},
            "load-old-init-procedure": "RULE_INIT",
            "*": "HTTP_REQUEST",
        }

    recorder = capture.emit(
        "tmm-RECORD", tag, "$nh_case", "$nh_stage", "$nh_code", "$nh_value", "$nh_unit"
    ).replace("|tmm-RECORD|", "|$nh_context|")
    record = (
        "proc record {nh_context nh_case nh_stage nh_code nh_value nh_invocation} {\nset nh_options_available unavailable\nset nh_return_options {}\nset nh_unit [TMM::cmp_group]:[TMM::cmp_unit]\n"
        + recorder
        + "\n}\n"
    )

    def measured(items, context):
        source = ""
        for item in items:
            source += "# |" + item["id"] + "|" + context + "|\n"
            source += (
                "set nh_setup "
                + capture.literal(item["setup"])
                + "\nset nh_operation "
                + capture.literal(item["operation"])
                + "\n"
            )
            source += (
                "set nh_c [catch {eval $nh_setup} nh_v]\ncall record "
                + context
                + " "
                + item["id"]
                + " setup $nh_c $nh_v $nh_invocation\n"
            )
            source += (
                "if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]\ncall record "
                + context
                + " "
                + item["id"]
                + " lookup $nh_c $nh_v $nh_invocation\n}\n"
            )
        return source

    provider_cells = f"""proc cells {{action invocation}} {{
 if {{$action == {{provider-write}}}} {{set {cells[0]} [list NAMESPACE_WRITE $invocation]; set {cells[1]} [list GLOBAL_WRITE $invocation]; set {cells[2]} [list STATIC_WRITE $invocation]}}
 set rows {{}}
 foreach cell [list {" ".join(cells)}] {{set c [catch {{set $cell}} v]; lappend rows $cell $c $v}}
 list rows $rows level [info level]
}}
proc link {{local_name invocation}} {{
 set provider_local [list PROVIDER_LOCAL $invocation]
 upvar 1 $local_name linked
 set before $linked
 set linked [list CALLER_WRITE $invocation]
 list local $provider_local before $before after $linked level [info level]
}}
"""

    def provider_config(owner, sentinel, bad=False):
        operation = (
            "__nh_" + tag + "_deliberately_absent"
            if bad
            else "set rows {}; foreach value [list $fixed $value] {binary scan $value H* h; lappend rows $h}; list owner "
            + sentinel
            + " argc 2 argv [list $fixed $value] argv-hex $rows"
        )
        return (
            "ltm rule "
            + owner
            + " {\nproc "
            + tail
            + " {fixed value} {"
            + operation
            + "}\n"
            + (provider_cells if owner == provider else "")
            + "}\n"
        )

    def observer_config(name, phase, initial=False):
        own = name if name.startswith("/") else folder + "/" + name
        measure = (
            "proc measure {nh_phase nh_invocation nh_action} {\n"
            + measured(cases, "tmm-procedure")
            + "}\n"
        )
        invocation = ("load-" if initial else "") + phase
        action = "provider-write" if initial else "read"
        init = (
            "set nh_phase "
            + phase
            + "\nset nh_invocation "
            + invocation
            + "-init\nset nh_action "
            + action
            + "\n"
        )
        if initial:
            init += (
                measured([seed_case], "tmm-RULE_INIT") + "if {$nh_c != 0} {return}\n"
            )
        init += (
            measured(cases, "tmm-RULE_INIT")
            + "call measure $nh_phase "
            + invocation
            + "-init-procedure "
            + action
            + "\n"
        )
        config = (
            "ltm rule "
            + own
            + " {\n"
            + record
            + "proc "
            + tail
            + " {fixed value} {list owner CALLER argc 2 argv [list $fixed $value]}\n"
            + measure
            + "when RULE_INIT {\n"
            + init
            + "}\n"
        )
        if initial:
            traffic = f'if {{![info exists {marker}] || [set {marker}] != "{tag}"}} {{HTTP::respond 503 content {{UNOWNED}} Connection close; return}}\n'
            traffic += "set nh_phase [HTTP::header value X-NH-Phase]\nif {$nh_phase == {}} {set nh_phase UNLABELLED_STANDARD_REQUEST}\nset nh_invocation [HTTP::header value X-NH-Invocation]\nset nh_action [HTTP::header value X-NH-Action]\n"
            traffic += (
                measured([*cases, connection_case], "tmm-traffic-event")
                + "call measure $nh_phase $nh_invocation $nh_action\n"
            )
            traffic += (
                'HTTP::respond 200 content "'
                + stem
                + ' [TMM::cmp_group]:[TMM::cmp_unit]" X-NH-TMM "[TMM::cmp_group]:[TMM::cmp_unit]" Connection keep-alive\n'
            )
            config += (
                "when CLIENT_ACCEPTED {\nif {[info exists "
                + conn
                + "]} {error {Connection cell collision}}\nset "
                + conn
                + " [list CONNECTION_SEED [IP::client_addr] [TCP::client_port]]\n}\nwhen HTTP_REQUEST {\n"
                + traffic
                + "}\n"
            )
        return config + "}\n", own

    initial, _ = observer_config(caller, "old", True)
    virtual = caller + "_vs"
    config = (
        "auth partition "
        + partition
        + " { default-route-domain 0 }\nsys folder "
        + folder
        + " {}\n"
    )
    config += "".join(
        provider_config(owner, sentinel)
        for owner, sentinel in zip(
            routes, ["OLD", "OTHER", "FOLDER", "PARTITION"], strict=True
        )
    )
    config += (
        initial
        + "ltm virtual "
        + virtual
        + " { destination 192.0.2.10:19912 mask 255.255.255.255 ip-protocol tcp profiles { tcp {} http {} } rules { "
        + provider
        + " "
        + other
        + " "
        + folded
        + " "
        + caller
        + " } }\n"
    )
    objects = [
        ("auth partition", partition),
        ("sys folder", folder),
        *[("ltm rule", owner) for owner in routes],
        ("ltm rule", caller),
        ("ltm virtual", virtual),
    ]
    return {
        "tag": tag,
        "stem": stem,
        "folder": folder,
        "provider": provider,
        "caller": caller,
        "virtual": virtual,
        "config": config,
        "objects": objects,
        "metadata": metadata,
        "preflight": preflight,
        "cleanup": cleanup_case,
        "cases": cases,
        "replacement": provider_config(provider, "NEW"),
        "rejected_replacement": provider_config(provider, "INVALID", True),
        "recreation": provider_config(provider, "RECREATED"),
        "observer_configs": {
            phase: observer_config(stem + "_observer_" + phase, phase)[0]
            for phase in PHASES
        },
        "limits": {
            "suspended_frames": "unmeasured",
            "native_catalog_retirement": "not inferred from configuration deletion",
            "traffic_workers": "actual response units only",
            "RULE_INIT_lifetime": "fresh observer loads after each phase",
            "connection_cleanup": "owned connections closed",
            "configuration_saved": False,
        },
    }


def install_private_transport():
    private_argv = list(capture.SSH)

    def rpc(command, data=None, timeout=180):
        result = subprocess.run(
            private_argv + [command],
            input=data,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        diagnostics = (
            b""
            if result.returncode == 0
            else b"Remote transport reported failure; connection diagnostics withheld.\n"
        )
        return subprocess.CompletedProcess(
            ["remote-operation"], result.returncode, result.stdout, diagnostics
        )

    capture.rpc = rpc
    capture.SSH = ["remote-operation", "connection-configuration-omitted"]


def save_rpc(dest, label, command, data=None):
    result = capture.rpc(command, data)
    (dest / (label + ".stdout")).write_bytes(result.stdout)
    (dest / (label + ".stderr")).write_bytes(result.stderr)
    (dest / (label + ".json")).write_text(
        json.dumps(
            {"remote_command": command, "exit_code": result.returncode}, indent=2
        )
        + "\n"
    )
    return result


def execute(plan, dest):
    install_private_transport()
    capture.TAG = plan["tag"]
    remote = "/var/tmp/tcl_lsp_" + plan["tag"]
    if save_rpc(
        dest,
        "staging-create",
        "test ! -e " + shlex.quote(remote) + " && mkdir -m 700 " + shlex.quote(remote),
    ).returncode:
        raise RuntimeError("Remote staging absence not proved")
    for label, command in [
        ("version", "tmsh show sys version"),
        ("platform", "tmsh show sys hardware"),
        ("tmm-roster", "tmsh show sys tmm-info"),
    ]:
        save_rpc(dest, label, command)
    preflight = dest / "tmm-RULE_INIT-0"
    capture.run_job(
        preflight,
        remote,
        "tmm-RULE_INIT",
        plan["stem"] + "_preflight",
        capture.body([plan["preflight"]], "tmm-RULE_INIT", plan["tag"]),
        wait=5,
    )
    fields = parse_capture(
        [preflight / "runtime-logs.stdout"], "tmm-RULE_INIT", plan["tag"]
    )["fields"]
    rows = [
        row
        for row in fields
        if row.get("case_id") == plan["preflight"]["id"]
        and row.get("stage") == "lookup"
        and row.get("field") == "result"
        and row.get("complete")
    ]
    if len(
        {row.get("tmm_unit") for row in rows if row.get("completion_code") == 0}
    ) != 4 or any(row.get("completion_code") != 0 for row in rows):
        raise RuntimeError("Four-unit owned state/native absence not proved")
    original_config = capture.make_config
    original_hook = capture.TRAFFIC_HOOK
    phase_state = {"phase": "old"}
    observers = []

    def observer(phase):
        index = 2 + PHASES.index(phase)
        job = dest / ("tmm-RULE_INIT-" + str(index))
        name = plan["stem"] + "_observer_" + phase
        owned = plan["folder"] + "/" + name
        before = capture.make_config
        try:
            config = plan["observer_configs"][phase]
            capture.make_config = lambda _c, _n, _s, port=19912: (
                config,
                [("ltm rule", owned)],
                None,
            )
            capture.run_job(job, remote, "tmm-RULE_INIT", name, config, wait=5)
        finally:
            capture.make_config = before
        observers.append(job)

    def request(conn, invocation, action="read"):
        conn.request(
            "GET",
            "/",
            headers={
                "X-NH-Invocation": invocation,
                "X-NH-Action": action,
                "X-NH-Phase": phase_state["phase"],
            },
        )
        response = conn.getresponse()
        body = response.read()
        row = {
            "invocation": invocation,
            "action": action,
            "phase": phase_state["phase"],
            "status": response.status,
            "unit": response.getheader("X-NH-TMM"),
            "body_hex": body.hex(),
            "run_marker_matches": plan["stem"].encode() in body,
        }
        if row["status"] != 200 or not row["run_marker_matches"] or not row["unit"]:
            raise RuntimeError("Owned response not proved")
        return row

    def paired(job):
        connections = []
        rows = []
        try:
            by_unit = {}
            pair = None
            for index in range(5):
                conn = http.client.HTTPConnection("192.0.2.10", 19912, timeout=3)
                connections.append(conn)
                row = request(conn, "pair-candidate-" + str(index))
                rows.append(row)
                if row["unit"] in by_unit:
                    pair = (by_unit[row["unit"]], conn)
                    break
                by_unit[row["unit"]] = conn
            if pair is None:
                rows.append(
                    {
                        "status": "unreached",
                        "reason": "No same-unit pair in five actual candidates",
                    }
                )
                return
            a, b = pair
            for conn, invocation, action in [
                (a, "pair-a-connection-write", "connection-write"),
                (b, "pair-b-connection-read", "read"),
                (a, "pair-a-connection-read", "read"),
                (a, "pair-a-provider-write", "provider-write"),
                (b, "pair-b-provider-read", "read"),
            ]:
                rows.append(request(conn, invocation, action))
            fresh = http.client.HTTPConnection("192.0.2.10", 19912, timeout=3)
            connections.append(fresh)
            rows.append(request(fresh, "pair-fresh-provider-read"))
        finally:
            for conn in connections:
                conn.close()
            (job / "paired-connections.json").write_text(
                json.dumps(rows, indent=2) + "\n"
            )

    def hook(job, staging, _context, _name, index):
        if index == 1:
            paired(job)
        phase_indexes = {
            0: "old",
            3: "new",
            6: "rejected",
            9: "detached",
            12: "deleted",
            15: "recreated",
        }
        if index in phase_indexes:
            phase = phase_indexes[index]
            if phase in {"new", "rejected", "recreated"}:
                key = {
                    "new": "replacement",
                    "rejected": "rejected_replacement",
                    "recreated": "recreation",
                }[phase]
                text = plan[key]
                (job / (phase + ".conf")).write_text(text)
                path = staging + "/" + phase + ".conf"
                if save_rpc(
                    job, phase + "-stage", "cat > " + shlex.quote(path), text.encode()
                ).returncode:
                    raise RuntimeError("Mutation staging failed")
                result = save_rpc(
                    job,
                    phase + "-apply",
                    "tmsh load sys config merge file " + shlex.quote(path),
                )
                if (phase == "rejected") == (result.returncode == 0):
                    raise RuntimeError(
                        "Mutation acceptance did not match phase requirement"
                    )
            elif phase == "detached":
                command = (
                    "tmsh modify ltm virtual "
                    + plan["virtual"]
                    + " rules { "
                    + plan["caller"]
                    + " }"
                )
                if save_rpc(job, phase + "-apply", command).returncode:
                    raise RuntimeError("Detachment failed")
            elif phase == "deleted":
                if save_rpc(
                    job, phase + "-apply", "tmsh delete ltm rule " + plan["provider"]
                ).returncode:
                    raise RuntimeError("Provider deletion failed")
            phase_state["phase"] = phase
            save_rpc(job, phase + "-provider", "tmsh list ltm rule " + plan["provider"])
            save_rpc(
                job, phase + "-attachment", "tmsh list ltm virtual " + plan["virtual"]
            )
            observer(phase)
        # capture.run_job sends its standard curl next; this additional labelled
        # request carries the phase headers, while the standard request is kept
        # as a separate invocation rather than assigned a false phase.
        conn = http.client.HTTPConnection("192.0.2.10", 19912, timeout=3)
        try:
            row = request(
                conn, "phase-" + str(index), "provider-write" if index == 0 else "read"
            )
            (job / ("phase-traffic-" + str(index) + ".json")).write_text(
                json.dumps(row, indent=2) + "\n"
            )
        finally:
            conn.close()

    job = dest / "tmm-traffic-event-0"
    try:
        capture.make_config = lambda _c, _n, _s, port=19912: (
            plan["config"],
            plan["objects"],
            None,
        )
        capture.TRAFFIC_HOOK = hook
        capture.run_job(
            job, remote, "tmm-traffic-event", plan["stem"], plan["config"], wait=5
        )
    finally:
        capture.make_config = original_config
        capture.TRAFFIC_HOOK = original_hook
        capture.run_job(
            dest / "tmm-RULE_INIT-9",
            remote,
            "tmm-RULE_INIT",
            plan["stem"] + "_cleanup",
            capture.body([plan["cleanup"]], "tmm-RULE_INIT", plan["tag"]),
            wait=5,
        )
    # Aliases identify the same actual execution; no duplicate execution claim.
    for source, context, number in [
        (job, "tmm-RULE_INIT", 1),
        (job, "tmm-procedure", 0),
        *[
            (source, "tmm-procedure", index + 1)
            for index, source in enumerate(observers)
        ],
    ]:
        alias = dest / (context + "-" + str(number))
        shutil.copytree(source, alias)
        (alias / "evidence-alias.json").write_text(
            json.dumps(
                {
                    "source_job": source.name,
                    "context_filter": context,
                    "independent_execution": False,
                },
                indent=2,
            )
            + "\n"
        )
    save_rpc(
        dest,
        "final-runtime",
        "grep -F " + shlex.quote("NHO|" + plan["tag"] + "|") + " /var/log/ltm",
    )
    (dest / "capture-complete.json").write_text(
        json.dumps({"completed": True, "configuration_saved": False}) + "\n"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", default="nh20261012native1")
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()
    plan = build_plan(args.run)
    dest = args.destination or Path(__file__).resolve().parent
    dest.mkdir(parents=True, exist_ok=True)
    if (dest / "plan.json").exists():
        raise RuntimeError("Preparation destination already contains a plan")
    if Path(__file__).resolve().parent != dest.resolve():
        (dest / "runner.py").write_bytes(Path(__file__).read_bytes())
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
                "remote_staging": "/var/tmp/tcl_lsp_" + args.run,
                "configuration_saved": False,
                "execution_requested": args.execute,
                "capture_helper": "scripts/dev/bigip-probes/name-patterns/capture.py",
                "capture_helper_sha256": capture.digest(
                    Path(capture.__file__).read_bytes()
                ),
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

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

"""Generate exact-byte fixtures for CONTEXT_NAMING_CHECKS.md."""

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def ascii_lf(text: str) -> bytes:
    data = text.encode("ascii")
    assert b"\r" not in data and b"\x00" not in data
    return data


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--run", required=True)
parser.add_argument("--out", required=True, type=Path)
parser.add_argument("--vip", required=True)
parser.add_argument("--backend", required=True)
parser.add_argument("--backend-port", required=True, type=int)
parser.add_argument("--vip-port", required=True, type=int)
parser.add_argument("--source-commit", required=True)
args = parser.parse_args()
if not re.fullmatch(r"[A-Za-z][A-Za-z0-9]{0,15}", args.run):
    parser.error("run must be ASCII alphanumeric and start with a letter")
if args.out.exists():
    parser.error("output directory already exists")

here = Path(__file__).resolve().parent
base = args.out / "base"
subprocess.run(
    [
        sys.executable,
        str(here / "generate-followup-controls.py"),
        "--run",
        args.run,
        "--out",
        str(base),
        "--vip",
        args.vip,
        "--backend",
        args.backend,
        "--backend-port",
        str(args.backend_port),
        "--vip-port",
        str(args.vip_port),
        "--source-commit",
        args.source_commit,
    ],
    check=True,
    stdout=subprocess.DEVNULL,
)

additional = args.out / "additional"
additional.mkdir()
contexts = args.out / "contexts"
contexts.mkdir()
events = args.out / "events"
events.mkdir()
lifecycles = args.out / "icall-lifecycles"
lifecycles.mkdir()
apl = args.out / "apl"
apl.mkdir()

canonical_cases = [
    "nul_counted_last",
    "nul_plain_last",
    "nul_plain_unset_first",
    "nul_array_root",
    "nul_array_index",
    "unicode_format_forward",
    "unicode_format_reverse",
    "unicode_bytes_forward",
    "unicode_bytes_reverse",
    "unicode_cross_producer",
    "commands_format",
    "commands_bytes",
    "commands_cross_producer",
    "lexical_format_unbraced",
    "lexical_format_braced",
    "lexical_bytes_unbraced",
    "lexical_bytes_braced",
]

lexical_clean = f"""set prefix __tcl_lsp_2286_{args.run}_lexclean_
set rows {{}}
foreach pair [list [list format_pre [format %c 233]] [list format_comb [format %c 769]] [list bytes_pre [binary format H* c3a9]] [list bytes_comb [binary format H* cc81]] [list nul [binary format H* 0042]]] {{
    set label [lindex $pair 0]
    set suffix [lindex $pair 1]
    set name $prefix
    append name $suffix
    set seed_rc [catch {{set $prefix SHORT_PREFIX}} seed_result]
    set write_rc [catch {{set $name FULL_VALUE}} write_result]
    foreach form {{unbraced braced}} {{
        set script "set observed \\$"
        if {{$form eq "braced"}} {{
            append script "\\{{" $name "\\}}"
        }} else {{
            append script $name
        }}
        binary scan $script H* script_hex
        set rc [catch {{eval $script}} result]
        binary scan $result H* result_hex
        lappend rows [list $label $form seed $seed_rc write $write_rc script_hex $script_hex eval $rc result_hex $result_hex]
    }}
    catch {{unset $name}}
    catch {{unset $prefix}}
    catch {{unset observed}}
}}
set rows
"""

qualified_nul = f"""set ns ::__tcl_lsp_2286_{args.run}_qualified
set traceproc ::__tcl_lsp_2286_{args.run}_qualified_trace
set rows {{}}
set trace_rows {{}}
set ns_create_rc [catch {{namespace eval $ns {{}}}} ns_create_result]
lappend rows [list namespace_create $ns_create_rc $ns_create_result]
set callback_body {{upvar 1 trace_rows trace_rows; binary scan $name1 H* root_hex; binary scan $name2 H* index_hex; lappend trace_rows [list $root_hex $index_hex $op]}}
catch {{eval [list rename $traceproc {{}}]}}
set trace_proc_rc [catch {{eval [list proc $traceproc {{name1 name2 op}} $callback_body]}} trace_proc_result]
lappend rows [list trace_proc $trace_proc_rc $trace_proc_result]
set plain $ns
append plain ::A
set counted $plain
append counted [binary format H* 0042]
binary scan $plain H* plain_hex
binary scan $counted H* counted_hex
lappend rows [list scalar_roots $plain_hex $counted_hex]
foreach pair [list [list $counted COUNTED] [list $plain PLAIN]] {{
    set rc [catch {{set [lindex $pair 0] [lindex $pair 1]}} result]
    lappend rows [list scalar_write $rc $result]
}}
set rc [catch {{upvar 0 $counted scalar_alias; set scalar_alias COUNTED_MUTATION}} result]
lappend rows [list scalar_upvar $rc $result]
set rc [catch {{unset $plain}} result]
lappend rows [list scalar_unset_plain $rc $result]
foreach name [list $plain $counted] {{
    set rc [catch {{set $name}} result]
    lappend rows [list scalar_after_unset $rc $result]
}}
catch {{unset scalar_alias}}
catch {{unset $plain}}
catch {{unset $counted}}
foreach root [list $plain $counted] {{
    set rc [catch {{trace variable $root rwu $traceproc}} result]
    lappend rows [list array_trace_add $rc $result]
}}
set plain_var $plain
append plain_var (k)
set counted_var $counted
append counted_var (k)
foreach pair [list [list $plain_var PLAIN_ARRAY] [list $counted_var COUNTED_ARRAY]] {{
    set rc [catch {{set [lindex $pair 0] [lindex $pair 1]}} result]
    lappend rows [list array_write $rc $result]
}}
set rc [catch {{upvar 0 $counted array_alias; set array_alias(k) COUNTED_ARRAY_MUTATION}} result]
lappend rows [list array_upvar $rc $result]
set rc [catch {{unset $counted}} result]
lappend rows [list array_unset_counted $rc $result]
foreach name [list $plain_var $counted_var] {{
    set rc [catch {{set $name}} result]
    lappend rows [list array_after_unset $rc $result]
}}
set index_root $ns
append index_root ::IndexRoot
set plain_index A
set counted_index $plain_index
append counted_index [binary format H* 0042]
set rc [catch {{trace variable $index_root rwu $traceproc}} result]
lappend rows [list index_trace_add $rc $result]
set plain_index_var $index_root
append plain_index_var ( $plain_index )
set counted_index_var $index_root
append counted_index_var ( $counted_index )
foreach pair [list [list $counted_index_var COUNTED_INDEX] [list $plain_index_var PLAIN_INDEX]] {{
    set rc [catch {{set [lindex $pair 0] [lindex $pair 1]}} result]
    lappend rows [list index_write $rc $result]
}}
set rc [catch {{upvar 0 $counted_index_var index_alias; set index_alias COUNTED_INDEX_MUTATION}} result]
lappend rows [list index_upvar $rc $result]
set rc [catch {{unset $plain_index_var}} result]
lappend rows [list index_unset_plain $rc $result]
foreach name [list $plain_index_var $counted_index_var] {{
    set rc [catch {{set $name}} result]
    lappend rows [list index_after_unset $rc $result]
}}
lappend rows [list trace_rows $trace_rows]
foreach root [list $plain $counted $index_root] {{catch {{trace vdelete $root rwu $traceproc}}; catch {{unset $root}}}}
catch {{eval [list rename $traceproc {{}}]}}
set ns_current_rc [catch {{namespace current}} ns_current_result]
set ns_which_rc [catch {{namespace which -command set}} ns_which_result]
lappend rows [list namespace_inventory $ns_current_rc $ns_current_result $ns_which_rc $ns_which_result]
catch {{namespace delete $ns}}
set rows
"""

environment = """set rows {}
foreach item {patchlevel tclversion} {
    set rc [catch {info $item} result]
    lappend rows [list info $item $rc $result]
}
set rc [catch {package provide Tcl} result]
lappend rows [list package_Tcl $rc $result]
foreach command {array binary catch encoding eval format info interp namespace package proc rename set trace unset uplevel upvar} {
    set rc [catch {info commands $command} result]
    lappend rows [list command $command $rc [llength $result] $result]
}
set rc [catch {lsort [info commands]} result]
lappend rows [list all_commands $rc $result]
set rows
"""

counted_input_boundaries = """when HTTP_REQUEST {
    set nul [format %c 0]
    set rows {}
    foreach spec {
        {short x}
        {long __resolution2286_context_invariance_long_ascii_prefix_0123456789}
        {different relocated_2286_key}
    } {
        set label [lindex $spec 0]
        set plain [lindex $spec 1]
        set name $plain
        append name $nul B $nul C
        catch {unset $plain}
        catch {unset $name}
        set plain_rc [catch {set $plain PLAIN} plain_result]
        set name_rc [catch {set $name COUNTED} name_result]
        foreach input [list $plain $name] {
            binary scan $input H* input_hex
            set rc [catch {set $input} result]
            binary scan $result H* result_hex
            lappend rows [list $label name_hex $input_hex write_plain $plain_rc write_counted $name_rc read $rc result_hex $result_hex]
        }
        catch {unset $name}
        set rc [catch {set $plain} result]
        binary scan $result H* result_hex
        lappend rows [list $label after_counted_unset $rc result_hex $result_hex]
        catch {unset $plain}
    }
    set inputs [list {} {()} {(k)} {A()} {A(k)} {A(k(l))} {A(k)tail} {:} {A:B} {A::B}]
    foreach input $inputs {
        binary scan $input H* input_hex
        set rc [catch {set $input GRAMMAR_VALUE} result]
        binary scan $result H* result_hex
        lappend rows [list grammar name_hex $input_hex write $rc result_hex $result_hex]
        set read_rc [catch {set $input} read_result]
        binary scan $read_result H* read_hex
        lappend rows [list grammar name_hex $input_hex read $read_rc result_hex $read_hex]
        catch {unset $input}
    }
    log local0.notice [list RESOLUTION2286_COUNTED_INPUT_BOUNDARIES tmm [TMM::cmp_unit] rows $rows]
    HTTP::respond 200 content OK
}
"""

counted_input_capture = counted_input_boundaries.replace(
    "    HTTP::respond 200 content OK\n",
    "    binary scan $rows H* rows_hex\n"
    "    HTTP::respond 200 content $rows_hex X-R2286-TMM \"[TMM::cmp_group]:[TMM::cmp_unit]\" Connection close\n",
)

array_root_boundaries = f"""set nul [format %c 0]
set rows {{}}
set trace_rows {{}}
set traceproc ::__tcl_lsp_2286_{args.run}_root_boundary_trace
set callback_body {{upvar 1 trace_rows trace_rows; binary scan $name1 H* root_hex; binary scan $name2 H* index_hex; lappend trace_rows [list $root_hex $index_hex $op]}}
catch {{eval [list rename $traceproc {{}}]}}
set trace_proc_rc [catch {{eval [list proc $traceproc {{name1 name2 op}} $callback_body]}} trace_proc_result]
lappend rows [list trace_proc $trace_proc_rc $trace_proc_result]
set ns ::__resolution2286_{args.run}_root_namespace
set ns_rc [catch {{namespace eval $ns {{}}}} ns_result]
lappend rows [list namespace_create $ns_rc $ns_result]
set pairs {{}}
foreach spec {{
    {{short relocated_root}}
    {{long __resolution2286_context_invariance_long_array_root_0123456789}}
    {{different elsewhere_2286_array_root}}
    {{colon root:colon}}
    {{parentheses root(k)}}
}} {{
    set label [lindex $spec 0]
    set plain [lindex $spec 1]
    set counted $plain
    append counted $nul B $nul C
    lappend pairs [list $label $plain $counted]
}}
set leading $nul
append leading B $nul C leading_root
lappend pairs [list leading {{}} $leading]
set middle relocated_
append middle $nul B $nul C middle_root
lappend pairs [list middle relocated_ $middle]
set qualified $ns
append qualified ::A
set qualified_counted $qualified
append qualified_counted $nul B $nul C
lappend pairs [list qualified $qualified $qualified_counted]
foreach pair $pairs {{
    set label [lindex $pair 0]
    set plain [lindex $pair 1]
    set counted [lindex $pair 2]
    binary scan $plain H* plain_hex
    binary scan $counted H* counted_hex
    foreach order {{plain_first counted_first}} {{
        catch {{unset root_alias}}
        catch {{trace vdelete $plain rwu $traceproc}}
        catch {{trace vdelete $counted rwu $traceproc}}
        catch {{unset $plain}}
        catch {{unset $counted}}
        set trace_rows {{}}
        set plain_trace_rc [catch {{trace variable $plain rwu $traceproc}} plain_trace_result]
        set counted_trace_rc [catch {{trace variable $counted rwu $traceproc}} counted_trace_result]
        set plain_var $plain
        append plain_var (k)
        set counted_var $counted
        append counted_var (k)
        if {{$order eq "plain_first"}} {{
            set writes [list [list $plain_var PLAIN] [list $counted_var COUNTED]]
            set unsets [list $counted $plain]
        }} else {{
            set writes [list [list $counted_var COUNTED] [list $plain_var PLAIN]]
            set unsets [list $plain $counted]
        }}
        set write_rows {{}}
        foreach write $writes {{
            set write_rc [catch {{set [lindex $write 0] [lindex $write 1]}} write_result]
            lappend write_rows [list $write_rc $write_result]
        }}
        set read_rows {{}}
        foreach input [list $plain_var $counted_var] {{
            set read_rc [catch {{set $input}} read_result]
            binary scan $read_result H* read_hex
            lappend read_rows [list $read_rc $read_hex]
        }}
        set upvar_rc [catch {{upvar 0 $counted root_alias; set root_alias(k) COUNTED_MUTATION}} upvar_result]
        set first_root [lindex $unsets 0]
        set survivor_root [lindex $unsets 1]
        set survivor_var $survivor_root
        append survivor_var (k)
        set unset_rc [catch {{unset $first_root}} unset_result]
        set survivor_rc [catch {{set $survivor_var}} survivor_result]
        binary scan $survivor_result H* survivor_hex
        lappend rows [list root $label order $order plain_hex $plain_hex counted_hex $counted_hex trace_add [list $plain_trace_rc $counted_trace_rc] writes $write_rows reads $read_rows upvar $upvar_rc $upvar_result unset_first $unset_rc $unset_result survivor $survivor_rc $survivor_hex traces $trace_rows]
    }}
    catch {{unset root_alias}}
    catch {{trace vdelete $plain rwu $traceproc}}
    catch {{trace vdelete $counted rwu $traceproc}}
    catch {{unset $plain}}
    catch {{unset $counted}}
}}
catch {{eval [list rename $traceproc {{}}]}}
catch {{namespace delete $ns}}
set rows
"""

array_index_boundaries = f"""set nul [format %c 0]
set rows {{}}
set trace_rows {{}}
set root __resolution2286_{args.run}_index_root
set traceproc ::__tcl_lsp_2286_{args.run}_index_boundary_trace
set callback_body {{upvar 1 trace_rows trace_rows; binary scan $name1 H* root_hex; binary scan $name2 H* index_hex; lappend trace_rows [list $root_hex $index_hex $op]}}
catch {{eval [list rename $traceproc {{}}]}}
set trace_proc_rc [catch {{eval [list proc $traceproc {{name1 name2 op}} $callback_body]}} trace_proc_result]
lappend rows [list trace_proc $trace_proc_rc $trace_proc_result]
set grammar_indices [list {{}} {{()}} {{(k)}} {{A()}} {{A(k)}} {{A(k(l))}} {{A(k)tail}} {{:}} {{A:B}} {{A::B}}]
foreach index $grammar_indices {{
    catch {{unset $root}}
    set trace_rows {{}}
    set trace_rc [catch {{trace variable $root rwu $traceproc}} trace_result]
    set variable $root
    append variable ( $index )
    binary scan $index H* index_hex
    set write_rc [catch {{set $variable GRAMMAR_VALUE}} write_result]
    set read_rc [catch {{set $variable}} read_result]
    binary scan $read_result H* read_hex
    set unset_rc [catch {{unset $variable}} unset_result]
    lappend rows [list grammar_index index_hex $index_hex trace_add $trace_rc write $write_rc $write_result read $read_rc $read_hex unset $unset_rc $unset_result traces $trace_rows]
    catch {{trace vdelete $root rwu $traceproc}}
    catch {{unset $root}}
}}
set pairs {{}}
foreach spec {{
    {{short q}}
    {{long __resolution2286_context_invariance_long_array_index_0123456789}}
    {{different elsewhere_2286_array_index}}
}} {{
    set label [lindex $spec 0]
    set plain [lindex $spec 1]
    set counted $plain
    append counted $nul B $nul C
    lappend pairs [list $label $plain $counted]
}}
set leading $nul
append leading B $nul C leading_index
lappend pairs [list leading {{}} $leading]
set middle relocated_
append middle $nul B $nul C middle_index
lappend pairs [list middle relocated_ $middle]
foreach pair $pairs {{
    set label [lindex $pair 0]
    set plain [lindex $pair 1]
    set counted [lindex $pair 2]
    binary scan $plain H* plain_hex
    binary scan $counted H* counted_hex
    foreach order {{plain_first counted_first}} {{
        catch {{unset index_alias}}
        catch {{trace vdelete $root rwu $traceproc}}
        catch {{unset $root}}
        set trace_rows {{}}
        set trace_rc [catch {{trace variable $root rwu $traceproc}} trace_result]
        set plain_var $root
        append plain_var ( $plain )
        set counted_var $root
        append counted_var ( $counted )
        if {{$order eq "plain_first"}} {{
            set writes [list [list $plain_var PLAIN] [list $counted_var COUNTED]]
            set unsets [list $counted_var $plain_var]
        }} else {{
            set writes [list [list $counted_var COUNTED] [list $plain_var PLAIN]]
            set unsets [list $plain_var $counted_var]
        }}
        set write_rows {{}}
        foreach write $writes {{
            set write_rc [catch {{set [lindex $write 0] [lindex $write 1]}} write_result]
            lappend write_rows [list $write_rc $write_result]
        }}
        set read_rows {{}}
        foreach input [list $plain_var $counted_var] {{
            set read_rc [catch {{set $input}} read_result]
            binary scan $read_result H* read_hex
            lappend read_rows [list $read_rc $read_hex]
        }}
        set upvar_rc [catch {{upvar 0 $counted_var index_alias; set index_alias COUNTED_MUTATION}} upvar_result]
        set unset_rc [catch {{unset [lindex $unsets 0]}} unset_result]
        set survivor_rc [catch {{set [lindex $unsets 1]}} survivor_result]
        binary scan $survivor_result H* survivor_hex
        lappend rows [list index $label order $order plain_hex $plain_hex counted_hex $counted_hex trace_add $trace_rc writes $write_rows reads $read_rows upvar $upvar_rc $upvar_result unset_first $unset_rc $unset_result survivor $survivor_rc $survivor_hex traces $trace_rows]
    }}
}}
catch {{unset index_alias}}
catch {{trace vdelete $root rwu $traceproc}}
catch {{unset $root}}
catch {{eval [list rename $traceproc {{}}]}}
set rows
"""


def portable_array_trace(case: str) -> bytes:
    payload = (base / f"{case}.tcl").read_text(encoding="ascii")
    payload = "set trace_rows {}\n" + re.sub(
        r"set callback_body \[binary format H\* [0-9a-f]+\]",
        "set callback_body {upvar 1 trace_rows trace_rows; "
        "binary scan $name1 H* root_hex; binary scan $name2 H* index_hex; "
        "lappend trace_rows [list $root_hex $index_hex $op]}",
        payload,
        count=1,
    )
    head, separator, tail = payload.rpartition("set rows\n")
    if not separator:
        raise RuntimeError(f"final set rows not found in {case}")
    payload = head + "lappend rows [list trace_rows $trace_rows]\n" + separator + tail
    return ascii_lf(payload)

additional_payloads = {
    "lexical_clean": ascii_lf(lexical_clean),
    "qualified_nul": ascii_lf(qualified_nul),
    "environment": ascii_lf(environment),
    "nul_array_root_portable": portable_array_trace("nul_array_root"),
    "nul_array_index_portable": portable_array_trace("nul_array_index"),
    "array_root_boundaries": ascii_lf(array_root_boundaries),
    "array_index_boundaries": ascii_lf(array_index_boundaries),
}
for case, data in additional_payloads.items():
    (additional / f"{case}.tcl").write_bytes(data)

counted_input_data = ascii_lf(counted_input_boundaries)
(additional / "counted_input_boundaries.tcl").write_bytes(counted_input_data)
counted_input_rule = ascii_lf(
    f"ltm rule /Common/__tcl_lsp_2286_{args.run}_counted_input_boundaries {{\n"
) + counted_input_data + b"}\n"
(additional / "counted_input_boundaries.conf").write_bytes(counted_input_rule)
(additional / "counted_input_boundaries.conf.hex").write_text(
    counted_input_rule.hex() + "\n", encoding="ascii"
)
counted_capture_data = ascii_lf(counted_input_capture)
(additional / "counted_input_capture.tcl").write_bytes(counted_capture_data)
counted_capture_rule = ascii_lf(
    f"ltm rule /Common/__tcl_lsp_2286_{args.run}_counted_input_capture {{\n"
) + counted_capture_data + b"}\n"
(additional / "counted_input_capture.conf").write_bytes(counted_capture_rule)
(additional / "counted_input_capture.conf.hex").write_text(
    counted_capture_rule.hex() + "\n", encoding="ascii"
)


def payload_path(case: str) -> Path:
    if case in additional_payloads:
        return additional / f"{case}.tcl"
    return base / f"{case}.tcl"


def write_http_wrapper(case: str, payload: bytes) -> dict[str, object]:
    rule = f"__tcl_lsp_2286_{args.run}_{case}"
    data = ascii_lf(
        f"""ltm rule /Common/{rule} {{
    when HTTP_REQUEST {{
        set payload [binary format H* {payload.hex()}]
        set rc [catch {{eval $payload}} result]
        binary scan $result H* result_hex
        log local0. "R2286NAMES|{args.run}|HTTP_REQUEST|{case}|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|rc=$rc|result_hex=$result_hex"
        HTTP::respond 200 content [list $rc $result_hex] X-R2286-TMM "[TMM::cmp_group]:[TMM::cmp_unit]" Connection close
    }}
}}
"""
    )
    path = additional / f"{case}.conf"
    path.write_bytes(data)
    (additional / f"{case}.conf.hex").write_text(data.hex() + "\n", encoding="ascii")
    return {
        "file": str(path.relative_to(args.out)),
        "sha256": digest(data),
        "size": len(data),
    }


http_wrappers = [
    write_http_wrapper(case, data) for case, data in additional_payloads.items()
]
http_wrappers.append(
    {
        "file": "additional/counted_input_boundaries.conf",
        "sha256": digest(counted_input_rule),
        "size": len(counted_input_rule),
    }
)
http_wrappers.append(
    {
        "file": "additional/counted_input_capture.conf",
        "sha256": digest(counted_capture_rule),
        "size": len(counted_capture_rule),
    }
)

event_wrappers = []
event_cases = ["nul_counted_last", "nul_array_root", "nul_array_index", "lexical_clean"]
for event in ("CLIENT_ACCEPTED", "RULE_INIT"):
    for case in event_cases:
        payload = payload_path(case).read_bytes()
        rule = f"__tcl_lsp_2286_{args.run}_{event.lower()}_{case}"
        data = ascii_lf(
            f"""ltm rule /Common/{rule} {{
    when {event} {{
        set payload [binary format H* {payload.hex()}]
        set rc [catch {{eval $payload}} result]
        binary scan $result H* result_hex
        set identity_rc [catch {{set tmm_group [TMM::cmp_group]; set tmm_unit [TMM::cmp_unit]}} identity_result]
        if {{$identity_rc != 0}} {{ set tmm_group UNAVAILABLE; set tmm_unit UNAVAILABLE }}
        log local0. "R2286NAMES|{args.run}|{event}|{case}|tmm=$tmm_group:$tmm_unit|identity_rc=$identity_rc|identity_result=$identity_result|rc=$rc|result_hex=$result_hex"
        set result_hex_length [string length $result_hex]
        set chunk_count [expr {{($result_hex_length + 399) / 400}}]
        for {{set chunk 0}} {{$chunk < $chunk_count}} {{incr chunk}} {{
            set first [expr {{$chunk * 400}}]
            set last [expr {{$first + 399}}]
            log local0. "R2286NAMESCHUNK|{args.run}|{event}|{case}|tmm=$tmm_group:$tmm_unit|rc=$rc|part=$chunk/$chunk_count|result_hex_chunk=[string range $result_hex $first $last]"
        }}
    }}
}}
"""
        )
        name = f"{event.lower()}-{case}.conf"
        (events / name).write_bytes(data)
        (events / f"{name}.hex").write_text(data.hex() + "\n", encoding="ascii")
        event_wrappers.append(
            {
                "event": event,
                "case": case,
                "file": f"events/{name}",
                "sha256": digest(data),
                "size": len(data),
            }
        )

context_rows = []
all_cases = canonical_cases + list(additional_payloads)
for index, case in enumerate(all_cases, start=1):
    context_run = f"{args.run}{index:02d}"
    out = contexts / case
    subprocess.run(
        [
            sys.executable,
            str(here / "generate-followup-contexts.py"),
            "--run",
            context_run,
            "--payload",
            str(payload_path(case)),
            "--out",
            str(out),
            "--source-commit",
            args.source_commit,
        ],
        check=True,
    )
    context_manifest = json.loads((out / "manifest.json").read_text(encoding="ascii"))
    context_rows.append(
        {
            "case": case,
            "context_run": context_run,
            "payload_sha256": context_manifest["payload"]["sha256"],
            "fixtures": context_manifest["fixtures"],
        }
    )

representative = [
    "nul_counted_last",
    "nul_array_root",
    "nul_array_index",
    "unicode_cross_producer",
    "lexical_clean",
]


def emit_lifecycle_rows(context: str) -> str:
    rows = []
    for case in representative:
        payload = payload_path(case).read_bytes()
        rows.append(
            f"""set payload [binary format H* {payload.hex()}]
set rc [catch {{eval $payload}} result]
binary scan $result H* result_hex
puts "R2286NAMES|{args.run}|{context}|{case}|payload_sha256={digest(payload)}|rc=$rc|result_hex=$result_hex"
"""
        )
    return "".join(rows)


periodic_script = f"/Common/__tcl_lsp_2286_{args.run}_periodic_script"
periodic_handler = f"/Common/__tcl_lsp_2286_{args.run}_periodic_handler"
periodic = ascii_lf(
    f"""sys icall script {periodic_script} {{
  definition {{
{emit_lifecycle_rows("ICallPeriodic")}  }}
}}
sys icall handler periodic {periodic_handler} {{
  script {periodic_script}
  interval 5
  last-occurrence now+1m
  status active
}}
"""
)
(lifecycles / "periodic.conf").write_bytes(periodic)

perpetual_script = f"/Common/__tcl_lsp_2286_{args.run}_perpetual_script"
perpetual_handler = f"/Common/__tcl_lsp_2286_{args.run}_perpetual_handler"
perpetual = ascii_lf(
    f"""sys icall script {perpetual_script} {{
  definition {{
set emitted 0
while {{1}} {{
    if {{!$emitted}} {{
{emit_lifecycle_rows("ICallPerpetual")}        set emitted 1
    }}
    EVENT::get_next -timeout 1000
}}
  }}
}}
sys icall handler perpetual {perpetual_handler} {{
  script {perpetual_script}
  status inactive
}}
"""
)
(lifecycles / "perpetual.conf").write_bytes(perpetual)

apl_payload = payload_path("lexical_clean").read_bytes()
apl_template = f"/Common/__tcl_lsp_2286_{args.run}_apl"
apl_service = f"/Common/__tcl_lsp_2286_{args.run}_apl_service"
apl_config = ascii_lf(
    f"""sys application template {apl_template} {{
  actions {{
    definition {{
      implementation {{
      }}
      presentation {{
        section probe {{
          choice naming_result default "UNREACHED" tcl {{
            set payload [binary format H* {apl_payload.hex()}]
            set rc [catch {{eval $payload}} result]
            binary scan $result H* result_hex
            puts "R2286NAMES|{args.run}|IAppPresentationTclCallback|lexical_clean|payload_sha256={digest(apl_payload)}|rc=$rc|result_hex=$result_hex"
            return [list [list "$rc:$result_hex" "$rc:$result_hex"]]
          }}
        }}
      }}
    }}
  }}
}}
"""
)
apl_service_config = ascii_lf(
    f"""sys application service {apl_service} {{
  template {apl_template}
}}
"""
)
(apl / "template.conf").write_bytes(apl_config)
(apl / "service.conf").write_bytes(apl_service_config)

payload_rows = []
for case in all_cases:
    path = payload_path(case)
    data = path.read_bytes()
    payload_rows.append(
        {
            "case": case,
            "file": str(path.relative_to(args.out)),
            "sha256": digest(data),
            "size": len(data),
            "ascii": data.isascii(),
            "contains_nul": b"\x00" in data,
            "contains_cr": b"\r" in data,
        }
    )
payload_rows.append(
    {
        "case": "counted_input_boundaries",
        "file": "additional/counted_input_boundaries.tcl",
        "sha256": digest(counted_input_data),
        "size": len(counted_input_data),
        "ascii": counted_input_data.isascii(),
        "contains_nul": b"\x00" in counted_input_data,
        "contains_cr": b"\r" in counted_input_data,
    }
)
payload_rows.append(
    {
        "case": "counted_input_capture",
        "file": "additional/counted_input_capture.tcl",
        "sha256": digest(counted_capture_data),
        "size": len(counted_capture_data),
        "ascii": counted_capture_data.isascii(),
        "contains_nul": b"\x00" in counted_capture_data,
        "contains_cr": b"\r" in counted_capture_data,
    }
)

manifest = {
    "run": args.run,
    "source_commit": args.source_commit,
    "vip": f"{args.vip}:{args.vip_port}",
    "backend": f"{args.backend}:{args.backend_port}",
    "base_manifest_sha256": digest((base / "manifest.json").read_bytes()),
    "payloads": payload_rows,
    "http_wrappers": http_wrappers,
    "event_wrappers": event_wrappers,
    "contexts": context_rows,
    "icall_lifecycles": [
        {
            "file": "icall-lifecycles/periodic.conf",
            "sha256": digest(periodic),
            "size": len(periodic),
        },
        {
            "file": "icall-lifecycles/perpetual.conf",
            "sha256": digest(perpetual),
            "size": len(perpetual),
        },
    ],
    "apl": [
        {
            "file": "apl/template.conf",
            "sha256": digest(apl_config),
            "size": len(apl_config),
        },
        {
            "file": "apl/service.conf",
            "sha256": digest(apl_service_config),
            "size": len(apl_service_config),
        },
    ],
}
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

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

"""Generate exact-byte BIG-IP name-identity and callable-lifetime controls."""

import argparse
import hashlib
import ipaddress
import json
import re
import subprocess
from pathlib import Path


def sha256(data: bytes) -> str:
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
parser.add_argument("--backend-port", type=int, required=True)
parser.add_argument("--vip-port", type=int, required=True)
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
if args.backend_port == args.vip_port:
    parser.error("backend and VIP ports must differ")
args.out.mkdir(parents=True)

source_commit = args.source_commit
if source_commit is None:
    source_commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()

prefix = f"__tcl_lsp_2286_{args.run}"
partition = f"R2286_{args.run}"
folder = f"/{partition}/lifetime"
provider = f"{folder}/{prefix}_provider"
caller = f"{folder}/{prefix}_caller"
fresh_provider = f"{folder}/{prefix}_provider_fresh"
fresh_caller = f"{folder}/{prefix}_caller_fresh"
backend_rule = f"/Common/{prefix}_backend"
pool = f"/{partition}/{prefix}_pool"
virtual = f"/{partition}/{prefix}_vs"
node = f"/Common/{prefix}_node"
manifest = {
    "run": args.run,
    "source_commit": source_commit,
    "line_endings": "LF",
    "payload_transport": "ASCII payload bytes transported as hex and evaluated dynamically",
    "objects": {
        "partition": partition,
        "folder": folder,
        "provider": provider,
        "caller": caller,
        "fresh_provider": fresh_provider,
        "fresh_caller": fresh_caller,
        "backend_rule": backend_rule,
        "pool": pool,
        "virtual": virtual,
        "node": node,
    },
    "payloads": [],
    "configs": [],
}


def write_file(name: str, data: bytes, kind: str, case: str) -> None:
    path = args.out / name
    path.write_bytes(data)
    manifest["configs" if kind == "config" else "payloads"].append(
        {
            "case": case,
            "file": name,
            "sha256": sha256(data),
            "size": len(data),
            "ascii": data.isascii(),
            "contains_nul": b"\x00" in data,
            "contains_cr": b"\r" in data,
        }
    )
    if kind == "config":
        (args.out / f"{name}.hex").write_text(data.hex() + "\n", encoding="ascii")


def add_payload(case: str, text: str) -> None:
    payload = ascii_lf(text)
    assert b"RUN_TOKEN" not in payload
    payload_name = f"{case}.tcl"
    write_file(payload_name, payload, "payload", case)
    rule = f"{prefix}_{case}"
    assert rule.replace("_", "").isalnum()
    body = (
        "ltm rule /Common/" + rule + " {\n"
        "    when HTTP_REQUEST {\n"
        "        set payload [binary format H* " + payload.hex() + "]\n"
        "        set rc [catch {eval $payload} result]\n"
        "        binary scan $result H* result_hex\n"
        '        log local0. "R2286 '
        + rule
        + ' tmm=[TMM::cmp_group]:[TMM::cmp_unit] rc=$rc result_hex=$result_hex"\n'
        '        HTTP::respond 200 content [list $rc $result_hex] X-R2286-TMM "[TMM::cmp_group]:[TMM::cmp_unit]" Connection close\n'
        "    }\n"
        "}\n"
    ).encode("ascii")
    write_file(f"{case}.conf", body, "config", case)


canonical_nul = """set prefix __tcl_lsp_2286_RUN_TOKEN_nul_
set plain ${prefix}A
set counted $plain
append counted [binary format H* 0042]
set rows [list [list names [string length $plain] [string length $counted]]]
foreach pair [list [list $plain PLAIN] [list $counted COUNTED]] {
    set name [lindex $pair 0]
    set value [lindex $pair 1]
    set rc [catch {set $name $value} result]
    lappend rows [list write [string length $name] $rc $result]
}
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list read [string length $name] $rc $result]
}
set rc [catch {upvar 0 $counted ${prefix}link; set ${prefix}link LINK_WRITE} result]
lappend rows [list upvar_write $rc $result]
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list after_link [string length $name] $rc $result]
}
set rc [catch {unset $counted} result]
lappend rows [list unset_counted $rc $result]
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list after_unset [string length $name] $rc $result]
}
foreach name [list $plain $counted ${prefix}link] {catch {unset $name}}
set rows
""".replace("RUN_TOKEN", args.run)
add_payload("nul_counted_last", canonical_nul)

add_payload(
    "nul_plain_last",
    canonical_nul.replace(
        "foreach pair [list [list $plain PLAIN] [list $counted COUNTED]]",
        "foreach pair [list [list $counted COUNTED] [list $plain PLAIN]]",
    ),
)

add_payload(
    "nul_plain_unset_first",
    canonical_nul.replace(
        "set rc [catch {unset $counted} result]\n"
        "lappend rows [list unset_counted $rc $result]",
        "set rc [catch {unset $plain} result]\n"
        "lappend rows [list unset_plain $rc $result]",
    ),
)


def nul_array_payload(case: str, root_mode: bool) -> str:
    if root_mode:
        setup = f"""set prefix __tcl_lsp_2286_{args.run}_arrayroot_
set plain $prefix
append plain A
set counted $plain
append counted [binary format H* 0042]
set plain_var $plain
append plain_var (k)
set counted_var $counted
append counted_var (k)
set procname ::__tcl_lsp_2286_{args.run}_{case}_trace
set rows [list [list roots [string length $plain] [string length $counted]]]
"""
        operations = """foreach item [list [list $plain_var PLAIN] [list $counted_var COUNTED]] {
    set varname [lindex $item 0]
    set rc [catch {set $varname [lindex $item 1]} result]
    lappend rows [list write [string length $varname] $rc $result]
}
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set label [lindex $item 0]
    set varname [lindex $item 1]
    lappend rows [list exists $label [info exists $varname]]
    set rc [catch {set $varname} result]
    lappend rows [list read $label $rc $result]
}
set rc [catch {upvar 0 $counted alias; set alias(k) COUNTED_MUTATION} result]
lappend rows [list upvar_write $rc $result]
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set rc [catch {set [lindex $item 1]} result]
    lappend rows [list after_link [lindex $item 0] $rc $result]
}
set rc [catch {unset $counted} result]
lappend rows [list unset_counted_root $rc $result]
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set varname [lindex $item 1]
    lappend rows [list after_unset_exists [lindex $item 0] [info exists $varname]]
    set rc [catch {set $varname} result]
    lappend rows [list after_unset_read [lindex $item 0] $rc $result]
}
foreach root [list $plain $counted] {catch {trace vdelete $root rwu $procname}; catch {unset $root}}
"""
        trace_targets = (
            "foreach root [list $plain $counted] {trace variable $root rwu $procname}"
        )
    else:
        setup = f"""set prefix __tcl_lsp_2286_{args.run}_arrayindex_
set root $prefix
append root root
set plain A
set counted $plain
append counted [binary format H* 0042]
set plain_var $root
append plain_var ( $plain )
set counted_var $root
append counted_var ( $counted )
set procname ::__tcl_lsp_2286_{args.run}_{case}_trace
set rows [list [list indexes [string length $plain] [string length $counted]]]
"""
        operations = """foreach item [list [list $plain_var PLAIN] [list $counted_var COUNTED]] {
    set varname [lindex $item 0]
    set rc [catch {set $varname [lindex $item 1]} result]
    lappend rows [list write [string length $varname] $rc $result]
}
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set label [lindex $item 0]
    set varname [lindex $item 1]
    lappend rows [list exists $label [info exists $varname]]
    set rc [catch {set $varname} result]
    lappend rows [list read $label $rc $result]
}
set rc [catch {upvar 0 $counted_var alias; set alias COUNTED_MUTATION} result]
lappend rows [list upvar_write $rc $result]
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set rc [catch {set [lindex $item 1]} result]
    lappend rows [list after_link [lindex $item 0] $rc $result]
}
set rc [catch {unset $counted_var} result]
lappend rows [list unset_counted_index $rc $result]
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set varname [lindex $item 1]
    lappend rows [list after_unset_exists [lindex $item 0] [info exists $varname]]
    set rc [catch {set $varname} result]
    lappend rows [list after_unset_read [lindex $item 0] $rc $result]
}
catch {trace vdelete $root rwu $procname}
catch {unset $root}
"""
        trace_targets = "trace variable $root rwu $procname"
    callback = f"""set callback_body [binary format H* 62696e617279207363616e20246e616d653120482a20726f6f745f6865780a62696e617279207363616e20246e616d653220482a20696e6465785f6865780a6c6f67206c6f63616c302e2022523232383654524143457c{args.run.encode().hex()}7c{case.encode().hex()}7c746d6d3d5b544d4d3a3a636d705f67726f75705d3a5b544d4d3a3a636d705f756e69745d7c726f6f745f6865783d24726f6f745f6865787c696e6465785f6865783d24696e6465785f6865787c6f703d246f7022]
catch {{eval [list rename $procname {{}}]}}
set rc [catch {{eval [list proc $procname {{name1 name2 op}} $callback_body]}} result]
lappend rows [list trace_proc $rc $result]
set rc [catch {{{trace_targets}}} result]
lappend rows [list trace_add $rc $result]
"""
    cleanup = """catch {eval [list rename $procname {}]}
set rows
"""
    return setup + callback + operations + cleanup


add_payload("nul_array_root", nul_array_payload("nul_array_root", True))
add_payload("nul_array_index", nul_array_payload("nul_array_index", False))

canonical_unicode = """set prefix __tcl_lsp_2286_RUN_TOKEN_unicode_
set pre $prefix
append pre [format %c 233]
set decomposed $prefix
append decomposed e [format %c 769]
set rows [list [list lengths [string length $pre] [string length $decomposed]]]
foreach name [list $pre $decomposed] {
    set raw_rc [catch {binary scan $name H* raw} raw_result]
    if {$raw_rc != 0} {set raw $raw_result}
    set rc [catch {encoding convertto utf-8 $name} encoded]
    if {$rc == 0} {binary scan $encoded H* encoded}
    lappend rows [list raw_conversion $raw_rc $raw utf8_conversion $rc $encoded]
}
set rc [catch {set $pre PRECOMPOSED} result]
lappend rows [list write_pre $rc $result]
set rc [catch {set $decomposed DECOMPOSED} result]
lappend rows [list write_decomposed $rc $result]
foreach name [list $pre $decomposed] {
    set rc [catch {set $name} result]
    lappend rows [list read $rc $result]
}
set rc [catch {unset $pre} result]
lappend rows [list unset_pre $rc $result]
set rc [catch {set $decomposed} result]
lappend rows [list remaining_decomposed $rc $result]
foreach name [list $pre $decomposed] {catch {unset $name}}
set rows
""".replace("RUN_TOKEN", args.run)
add_payload("unicode_format_forward", canonical_unicode)

unicode_reverse = canonical_unicode.replace(
    "set rc [catch {set $pre PRECOMPOSED} result]\n"
    "lappend rows [list write_pre $rc $result]\n"
    "set rc [catch {set $decomposed DECOMPOSED} result]\n"
    "lappend rows [list write_decomposed $rc $result]",
    "set rc [catch {set $decomposed DECOMPOSED} result]\n"
    "lappend rows [list write_decomposed $rc $result]\n"
    "set rc [catch {set $pre PRECOMPOSED} result]\n"
    "lappend rows [list write_pre $rc $result]",
).replace(
    "set rc [catch {unset $pre} result]\n"
    "lappend rows [list unset_pre $rc $result]\n"
    "set rc [catch {set $decomposed} result]\n"
    "lappend rows [list remaining_decomposed $rc $result]",
    "set rc [catch {unset $decomposed} result]\n"
    "lappend rows [list unset_decomposed $rc $result]\n"
    "set rc [catch {set $pre} result]\n"
    "lappend rows [list remaining_pre $rc $result]",
)
add_payload("unicode_format_reverse", unicode_reverse)


def unicode_bytes(reverse: bool) -> str:
    first = "decomposed" if reverse else "pre"
    second = "pre" if reverse else "decomposed"
    first_value = "DECOMPOSED" if reverse else "PRECOMPOSED"
    second_value = "PRECOMPOSED" if reverse else "DECOMPOSED"
    return f"""set prefix __tcl_lsp_2286_{args.run}_unicodebytes_
set pre $prefix
append pre [binary format H* c3a9]
set decomposed $prefix
append decomposed [binary format H* 65cc81]
set rows [list [list lengths [string length $pre] [string length $decomposed]]]
foreach name [list $pre $decomposed] {{
    set raw_rc [catch {{binary scan $name H* raw}} raw_result]
    if {{$raw_rc != 0}} {{set raw $raw_result}}
    set rc [catch {{encoding convertto utf-8 $name}} encoded]
    if {{$rc == 0}} {{binary scan $encoded H* encoded}}
    lappend rows [list raw_conversion $raw_rc $raw utf8_conversion $rc $encoded]
}}
set rc [catch {{set ${first} {first_value}}} result]
lappend rows [list write_{first} $rc $result]
set rc [catch {{set ${second} {second_value}}} result]
lappend rows [list write_{second} $rc $result]
foreach name [list $pre $decomposed] {{
    set rc [catch {{set $name}} result]
    lappend rows [list read $rc $result]
}}
set rc [catch {{unset ${first}}} result]
lappend rows [list unset_{first} $rc $result]
set rc [catch {{set ${second}}} result]
lappend rows [list remaining_{second} $rc $result]
foreach name [list $pre $decomposed] {{catch {{unset $name}}}}
set rows
"""


add_payload("unicode_bytes_forward", unicode_bytes(False))
add_payload("unicode_bytes_reverse", unicode_bytes(True))

add_payload(
    "unicode_cross_producer",
    f"""set prefix __tcl_lsp_2286_{args.run}_unicodecross_
set format_pre $prefix
append format_pre [format %c 233]
set bytes_pre $prefix
append bytes_pre [binary format H* c3a9]
set format_decomposed $prefix
append format_decomposed e [format %c 769]
set bytes_decomposed $prefix
append bytes_decomposed [binary format H* 65cc81]
set rows {{}}
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {{
    set label [lindex $pair 0]
    set name [lindex $pair 1]
    binary scan $name H* raw
    lappend rows [list representation $label [string length $name] $raw]
}}
foreach pair [list [list $format_pre FORMAT_PRE] [list $bytes_pre BYTES_PRE] [list $format_decomposed FORMAT_DECOMPOSED] [list $bytes_decomposed BYTES_DECOMPOSED]] {{
    set rc [catch {{set [lindex $pair 0] [lindex $pair 1]}} result]
    lappend rows [list write [lindex $pair 1] $rc $result]
}}
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {{
    set rc [catch {{set [lindex $pair 1]}} result]
    lappend rows [list read [lindex $pair 0] $rc $result]
}}
set rc [catch {{upvar 0 $bytes_pre pre_alias; set pre_alias BYTES_PRE_MUTATION}} result]
lappend rows [list mutate_bytes_pre $rc $result]
set rc [catch {{upvar 0 $bytes_decomposed decomposed_alias; set decomposed_alias BYTES_DECOMPOSED_MUTATION}} result]
lappend rows [list mutate_bytes_decomposed $rc $result]
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {{
    set rc [catch {{set [lindex $pair 1]}} result]
    lappend rows [list after_mutation [lindex $pair 0] $rc $result]
}}
set rc [catch {{unset $format_pre}} result]
lappend rows [list unset_format_pre $rc $result]
set rc [catch {{unset $format_decomposed}} result]
lappend rows [list unset_format_decomposed $rc $result]
foreach pair [list [list bytes_pre $bytes_pre] [list bytes_decomposed $bytes_decomposed]] {{
    set rc [catch {{set [lindex $pair 1]}} result]
    lappend rows [list after_format_unset [lindex $pair 0] $rc $result]
}}
foreach name [list $format_pre $bytes_pre $format_decomposed $bytes_decomposed] {{catch {{unset $name}}}}
set rows
""",
)


def lexical_payload(producer: str, braced: bool) -> str:
    if producer == "format":
        make = """set pre $prefix
append pre [format %c 233]
set decomposed $prefix
append decomposed e [format %c 769]
"""
    else:
        make = """set pre $prefix
append pre [binary format H* c3a9]
set decomposed $prefix
append decomposed [binary format H* 65cc81]
"""
    token = (
        'set script "set \\${"; append script $name; append script "}"'
        if braced
        else 'set script "set \\$"; append script $name'
    )
    return f"""set prefix __tcl_lsp_2286_{args.run}_lex_{producer}_
{make}set rows {{}}
foreach pair [list [list pre $pre PRE_VALUE] [list decomposed $decomposed DECOMPOSED_VALUE]] {{
    set label [lindex $pair 0]
    set name [lindex $pair 1]
    set value [lindex $pair 2]
    set write_rc [catch {{set $name $value}} write_result]
    {token}
    binary scan $script H* script_hex
    set rc [catch {{eval $script}} result]
    binary scan $result H* result_hex
    lappend rows [list $label write $write_rc $write_result script_hex $script_hex eval $rc $result_hex]
}}
foreach name [list $pre $decomposed] {{catch {{unset $name}}}}
set rows
"""


for producer in ("format", "bytes"):
    add_payload(f"lexical_{producer}_unbraced", lexical_payload(producer, False))
    add_payload(f"lexical_{producer}_braced", lexical_payload(producer, True))


def command_payload(producer: str) -> str:
    if producer == "format":
        make = """set pre $prefix
append pre [format %c 233]
set decomposed $prefix
append decomposed e [format %c 769]
"""
    else:
        make = """set pre $prefix
append pre [binary format H* c3a9]
set decomposed $prefix
append decomposed [binary format H* 65cc81]
"""
    return f"""set prefix ::__tcl_lsp_2286_{args.run}_command_{producer}_
{make}set rows {{}}
foreach pair [list [list pre $pre PRE_COMMAND] [list decomposed $decomposed DECOMPOSED_COMMAND]] {{
    set label [lindex $pair 0]
    set ns [lindex $pair 1]
    set value [lindex $pair 2]
    set procname $ns
    append procname ::identity
    set renamed $procname
    append renamed _renamed
    binary scan $ns H* ns_hex
    set rc [catch {{namespace eval $ns {{}}}} result]
    lappend rows [list namespace_create $label $ns_hex $rc $result]
    set body [list return $value]
    set rc [catch {{eval [list proc $procname {{}} $body]}} result]
    lappend rows [list proc_create $label $rc $result]
    set rc [catch {{namespace which -command $procname}} result]
    lappend rows [list lookup_original $label $rc $result]
    set rc [catch {{eval [list $procname]}} result]
    lappend rows [list call_original $label $rc $result]
    set rc [catch {{rename $procname $renamed}} result]
    lappend rows [list rename $label $rc $result]
    set rc [catch {{namespace which -command $procname}} result]
    lappend rows [list lookup_old_after_rename $label $rc $result]
    set rc [catch {{namespace which -command $renamed}} result]
    lappend rows [list lookup_new_after_rename $label $rc $result]
    set rc [catch {{eval [list $renamed]}} result]
    lappend rows [list call_renamed $label $rc $result]
}}
foreach ns [list $pre $decomposed] {{catch {{namespace delete $ns}}}}
set rows
"""


add_payload("commands_format", command_payload("format"))
add_payload("commands_bytes", command_payload("bytes"))

add_payload(
    "commands_cross_producer",
    f"""set prefix ::__tcl_lsp_2286_{args.run}_command_cross_
set format_pre $prefix
append format_pre [format %c 233]
set bytes_pre $prefix
append bytes_pre [binary format H* c3a9]
set format_decomposed $prefix
append format_decomposed e [format %c 769]
set bytes_decomposed $prefix
append bytes_decomposed [binary format H* 65cc81]
set rows {{}}
foreach group [list [list pre $format_pre $bytes_pre] [list decomposed $format_decomposed $bytes_decomposed]] {{
    set label [lindex $group 0]
    set format_ns [lindex $group 1]
    set bytes_ns [lindex $group 2]
    set format_proc $format_ns
    append format_proc ::identity
    set bytes_proc $bytes_ns
    append bytes_proc ::identity
    set renamed $bytes_proc
    append renamed _renamed
    foreach pair [list [list format $format_ns] [list bytes $bytes_ns]] {{
        set ns [lindex $pair 1]
        binary scan $ns H* raw
        set rc [catch {{namespace eval $ns {{}}}} result]
        lappend rows [list namespace_create $label [lindex $pair 0] [string length $ns] $raw $rc $result]
    }}
    set rc [catch {{eval [list proc $format_proc {{}} [list return FORMAT_COMMAND]]}} result]
    lappend rows [list proc_create $label format $rc $result]
    set rc [catch {{eval [list proc $bytes_proc {{}} [list return BYTES_COMMAND]]}} result]
    lappend rows [list proc_create $label bytes $rc $result]
    foreach pair [list [list format $format_proc] [list bytes $bytes_proc]] {{
        set name [lindex $pair 1]
        set lookup_rc [catch {{namespace which -command $name}} lookup]
        set call_rc [catch {{eval [list $name]}} value]
        lappend rows [list before_rename $label [lindex $pair 0] $lookup_rc $lookup $call_rc $value]
    }}
    set rc [catch {{rename $bytes_proc $renamed}} result]
    lappend rows [list rename_bytes $label $rc $result]
    foreach pair [list [list format $format_proc] [list bytes $bytes_proc] [list renamed $renamed]] {{
        set name [lindex $pair 1]
        set lookup_rc [catch {{namespace which -command $name}} lookup]
        set call_rc [catch {{eval [list $name]}} value]
        lappend rows [list after_rename $label [lindex $pair 0] $lookup_rc $lookup $call_rc $value]
    }}
}}
foreach ns [list $format_pre $bytes_pre $format_decomposed $bytes_decomposed] {{catch {{namespace delete $ns}}}}
set rows
""",
)

add_payload(
    "expressions",
    """set rows {}
foreach expression {
    {"abcd" matches "a*"}
    {"a*" matches "a*"}
    {"a*" matches "abcd"}
    {"abcd" matches "a.*"}
    {"abcd" matches "bc"}
    {"abcd" matches "abcd"}
    {1 or 0 matches 0}
    {not "abc" starts_with "a"}
    {not ("abc" starts_with "a")}
} {
    set rc [catch {expr $expression} result]
    lappend rows [list $expression $rc $result]
}
set rows
""",
)

backend_body = f"""ltm rule {backend_rule} {{
    when HTTP_REQUEST {{
        HTTP::header insert X-R2286-Scope {args.run}-backend
        log local0. "R2286BACKEND|{args.run}|request=[HTTP::header value X-R2286-Request]|tmm=[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
    when HTTP_RESPONSE {{
        HTTP::header insert X-R2286-TMM "[TMM::cmp_group]:[TMM::cmp_unit]"
    }}
}}
"""
write_file("backend.conf", ascii_lf(backend_body), "config", "backend")


def provider_text(
    label: str,
    dynamic_namespace: bool,
    new_formals: bool,
    owner: str = provider,
) -> str:
    formals = "{name {suffix NEW_DEFAULT}}" if new_formals else "{name}"
    set_cell = "NEW_WRITE" if new_formals else "OLD_WRITE"
    if dynamic_namespace:
        namespace_lines = (
            "    set ns_command [binary format H* 6e616d6573706163652063757272656e74]\n"
            "    set ns [eval $ns_command]\n"
        )
        ns_value = "$ns"
    else:
        namespace_lines = ""
        ns_value = "[namespace current]"
    if new_formals:
        returned = f"[list {label} $suffix {ns_value} $cell]"
    else:
        returned = f"[list {label} {ns_value} $cell]"
    return f"""ltm rule {owner} {{
proc incarnation {formals} {{
    upvar 1 $name cell
    set cell {set_cell}
{namespace_lines}    return {returned}
}}
}}
"""


for name, label, dynamic, new_formals in (
    ("provider-old-literal.conf", "OLD", False, False),
    ("provider-new-literal.conf", "NEW", False, True),
    ("provider-old-dynamic.conf", "OLD", True, False),
    ("provider-new-dynamic.conf", "NEW", True, True),
):
    write_file(
        name,
        ascii_lf(provider_text(label, dynamic, new_formals)),
        "config",
        name.removesuffix(".conf"),
    )

failed_provider = f"""ltm rule {provider} {{
proc incarnation {{name}} {{
    upvar 1 $name cell
    set cell FAILED_REPLACEMENT_MUST_NOT_RUN
    return [list FAILED
}}
}}
"""
write_file(
    "provider-failed-replacement.conf",
    ascii_lf(failed_provider),
    "config",
    "provider-failed-replacement",
)

target = f"{provider}::incarnation"
caller_body = f"""ltm rule {caller} {{
when HTTP_REQUEST {{
    set phase [HTTP::header value X-R2286-Phase]
    set request [HTTP::header value X-R2286-Request]
    set cell CALLER_SEED
    if {{[HTTP::path] eq "/two"}} {{
        set rc [catch {{call {target} cell EXPLICIT_SUFFIX}} result]
        set actuals two
    }} else {{
        set rc [catch {{call {target} cell}} result]
        set actuals one
    }}
    binary scan $result H* result_hex
    binary scan $cell H* cell_hex
    log local0. "R2286LIFE|{args.run}|phase=$phase|request=$request|actuals=$actuals|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|rc=$rc|result_hex=$result_hex|cell_hex=$cell_hex"
    HTTP::respond 200 content [list $rc $result_hex $cell_hex] X-R2286-TMM "[TMM::cmp_group]:[TMM::cmp_unit]" Connection close
}}
}}
"""
write_file("caller.conf", ascii_lf(caller_body), "config", "caller")

dynamic_target = target.encode("ascii").hex()
caller_dynamic_body = f"""ltm rule {caller} {{
when HTTP_REQUEST {{
    set phase [HTTP::header value X-R2286-Phase]
    set request [HTTP::header value X-R2286-Request]
    set cell CALLER_SEED
    set target [binary format H* {dynamic_target}]
    if {{[HTTP::path] eq "/two"}} {{
        set rc [catch {{call $target cell EXPLICIT_SUFFIX}} result]
        set actuals two
    }} else {{
        set rc [catch {{call $target cell}} result]
        set actuals one
    }}
    binary scan $result H* result_hex
    binary scan $cell H* cell_hex
    log local0. "R2286LIFEDYNAMIC|{args.run}|phase=$phase|request=$request|actuals=$actuals|target=$target|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|rc=$rc|result_hex=$result_hex|cell_hex=$cell_hex"
    HTTP::respond 200 content [list $rc $result_hex $cell_hex] X-R2286-TMM "[TMM::cmp_group]:[TMM::cmp_unit]" Connection close
}}
}}
"""
write_file(
    "caller-dynamic.conf",
    ascii_lf(caller_dynamic_body),
    "config",
    "caller-dynamic",
)

caller_header_body = f"""ltm rule {caller} {{
when HTTP_REQUEST {{
    set phase [HTTP::header value X-R2286-Phase]
    set request [HTTP::header value X-R2286-Request]
    set cell CALLER_SEED
    set target [HTTP::header value X-R2286-Target]
    if {{[HTTP::path] eq "/two"}} {{
        set rc [catch {{call $target cell EXPLICIT_SUFFIX}} result]
        set actuals two
    }} else {{
        set rc [catch {{call $target cell}} result]
        set actuals one
    }}
    binary scan $result H* result_hex
    binary scan $cell H* cell_hex
    log local0. "R2286LIFEHEADER|{args.run}|phase=$phase|request=$request|actuals=$actuals|target=$target|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|rc=$rc|result_hex=$result_hex|cell_hex=$cell_hex"
    HTTP::respond 200 content [list $rc $result_hex $cell_hex] X-R2286-TMM "[TMM::cmp_group]:[TMM::cmp_unit]" Connection close
}}
}}
"""
write_file(
    "caller-header-target.conf",
    ascii_lf(caller_header_body),
    "config",
    "caller-header-target",
)

for name, label, new_formals in (
    ("provider-fresh-old-dynamic.conf", "OLD", False),
    ("provider-fresh-new-dynamic.conf", "NEW", True),
):
    write_file(
        name,
        ascii_lf(provider_text(label, True, new_formals, fresh_provider)),
        "config",
        name.removesuffix(".conf"),
    )

fresh_failed_provider = failed_provider.replace(provider, fresh_provider)
write_file(
    "provider-fresh-failed-replacement.conf",
    ascii_lf(fresh_failed_provider),
    "config",
    "provider-fresh-failed-replacement",
)

fresh_caller_header_body = caller_header_body.replace(caller, fresh_caller).replace(
    "R2286LIFEHEADER", "R2286LIFEFRESH"
)
write_file(
    "caller-fresh-header-target.conf",
    ascii_lf(fresh_caller_header_body),
    "config",
    "caller-fresh-header-target",
)

lab = f"""ltm node {node} {{
    address {backend}
}}
ltm pool {pool} {{
    members {{ {node}:{args.backend_port} {{ address {backend} }} }}
}}
ltm virtual {virtual} {{
    destination {vip}:{args.vip_port}
    mask 255.255.255.255
    ip-protocol tcp
    source 0.0.0.0/0
    profiles {{ /Common/tcp {{ }} /Common/http {{ }} }}
    pool {pool}
    rules {{ {backend_rule} }}
    source-address-translation {{ type automap }}
    cmp-enabled yes
}}
"""
write_file("lab.conf", ascii_lf(lab), "config", "lab")

(args.out / "manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
print(json.dumps(manifest, indent=2, sort_keys=True))

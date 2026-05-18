#!/usr/bin/env python3
"""Audit OptionSpec dialect gates against real tclsh 8.4/8.5/8.6/9.0.

For each option in the registry, generate a probe script and run it against
each built tclsh.  An option is supported if the probe runs without
hitting the "bad option" / "unknown option" error or a syntax-level
rejection.  Records the per-version availability so the registry can be
updated with accurate ``dialects`` frozensets.

Run from repo root:
    uv run --extra dev python scripts/audit_option_dialects.py
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

TCL_DIRS = {
    "tcl8.4": ROOT / "tmp" / "tcl8.4.20" / "unix",
    "tcl8.5": ROOT / "tmp" / "tcl8.5.19" / "unix",
    "tcl8.6": ROOT / "tmp" / "tcl8.6.16" / "unix",
    "tcl9.0": ROOT / "tmp" / "tcl9.0.3" / "unix",
}

# Per-option probe scripts.  Each entry maps (command, subcommand_or_None,
# option_name) -> Tcl probe script.  The probe should exercise the option
# in a way that gives a definitive answer:
#   * Returns 0 / "ok" / nothing → option accepted (lexical/dispatch wise).
#   * "bad option" / "unknown option" / "invalid option" / "wrong # args" /
#     "no such option" → option NOT accepted.
#   * Channel / permission / runtime errors are still "accepted" — the
#     command parsed and the option was recognised.
#
# We test via ``catch`` and inspect ``$err`` to discriminate.
PROBES: dict[tuple[str, str | None, str], str] = {
    # ---- lsearch ----
    ("lsearch", None, "-stride"): "lsearch -stride 2 {a 1} *",
    ("lsearch", None, "-bisect"): "lsearch -bisect -sorted -decreasing {3 2 1} 2",
    ("lsearch", None, "-subindices"): "lsearch -subindices -index 0 {a b} a",
    ("lsearch", None, "-index"): "lsearch -index 0 {a b} a",
    ("lsearch", None, "-inline"): "lsearch -inline {a} a",
    ("lsearch", None, "-not"): "lsearch -not {a b} a",
    ("lsearch", None, "-start"): "lsearch -start 0 {a b} b",
    # ---- lsort ----
    ("lsort", None, "-stride"): "lsort -stride 2 {a 1 b 2}",
    ("lsort", None, "-indices"): "lsort -indices {a b c}",
    ("lsort", None, "-unique"): "lsort -unique {a b c}",
    ("lsort", None, "-command"): "lsort -command {string compare} {a b c}",
    # ---- regsub ----
    ("regsub", None, "-command"): "regsub -command {a} {abc} {string toupper}",
    ("regsub", None, "-expanded"): "regsub -expanded {a} {abc} X",
    ("regsub", None, "-line"): "regsub -line {a} {abc} X",
    ("regsub", None, "-linestop"): "regsub -linestop {a} {abc} X",
    ("regsub", None, "-lineanchor"): "regsub -lineanchor {a} {abc} X",
    ("regsub", None, "-all"): "regsub -all {a} {aaa} X",
    ("regsub", None, "-start"): "regsub -start 0 {a} {abc} X",
    # ---- regexp ----
    ("regexp", None, "-expanded"): "regexp -expanded {a} {abc}",
    ("regexp", None, "-line"): "regexp -line {a} {abc}",
    ("regexp", None, "-linestop"): "regexp -linestop {a} {abc}",
    ("regexp", None, "-lineanchor"): "regexp -lineanchor {a} {abc}",
    ("regexp", None, "-all"): "regexp -all {a} {aaa}",
    ("regexp", None, "-inline"): "regexp -inline {a} {abc}",
    ("regexp", None, "-indices"): "regexp -indices {a} {abc}",
    ("regexp", None, "-start"): "regexp -start 0 {a} {abc}",
    ("regexp", None, "-about"): "regexp -about {a}",
    # ---- exec ----
    ("exec", None, "-ignorestderr"): "exec -ignorestderr -- echo hi",
    ("exec", None, "-keepnewline"): "exec -keepnewline -- echo hi",
    # ---- glob ----
    ("glob", None, "-directory"): "glob -directory /tmp -nocomplain *",
    ("glob", None, "-join"): "glob -join -nocomplain /tmp *",
    ("glob", None, "-path"): "glob -path /tmp/x -nocomplain *",
    ("glob", None, "-tails"): "glob -tails -directory /tmp -nocomplain *",
    ("glob", None, "-types"): "glob -types f -directory /tmp -nocomplain *",
    ("glob", None, "-nocomplain"): "glob -nocomplain /nonexistent/x/*",
    # ---- file copy / delete / rename / link ----
    ("file", "copy", "-force"): "file copy -force -- /tmp/_audit_a /tmp/_audit_b",
    ("file", "delete", "-force"): "file delete -force -- /tmp/_audit_z",
    ("file", "rename", "-force"): "file rename -force -- /tmp/_audit_a /tmp/_audit_c",
    ("file", "link", "-symbolic"): "file link -symbolic /tmp/_audit_link /tmp/_audit_target",
    ("file", "link", "-hard"): "file link -hard /tmp/_audit_link /tmp/_audit_target",
    # ---- chan / fconfigure (channel options) ----
    # These need a real channel; using a memory channel via [open] on a tempfile.
    (
        "fconfigure",
        None,
        "-blocking",
    ): "set f [open /tmp/_audit_ch w]; fconfigure $f -blocking 0; close $f",
    (
        "fconfigure",
        None,
        "-buffering",
    ): "set f [open /tmp/_audit_ch w]; fconfigure $f -buffering line; close $f",
    (
        "fconfigure",
        None,
        "-buffersize",
    ): "set f [open /tmp/_audit_ch w]; fconfigure $f -buffersize 4096; close $f",
    (
        "fconfigure",
        None,
        "-encoding",
    ): "set f [open /tmp/_audit_ch w]; fconfigure $f -encoding utf-8; close $f",
    (
        "fconfigure",
        None,
        "-eofchar",
    ): "set f [open /tmp/_audit_ch w]; fconfigure $f -eofchar {}; close $f",
    (
        "fconfigure",
        None,
        "-translation",
    ): "set f [open /tmp/_audit_ch w]; fconfigure $f -translation auto; close $f",
    (
        "fconfigure",
        None,
        "-profile",
    ): "set f [open /tmp/_audit_ch w]; fconfigure $f -profile strict; close $f",
    (
        "fconfigure",
        None,
        "-keepalive",
    ): "set s [socket -server {} -myaddr 127.0.0.1 0]; set p [lindex [fconfigure $s -sockname] 2]; set c [socket 127.0.0.1 $p]; fconfigure $c -keepalive 1; close $c; close $s",
    (
        "fconfigure",
        None,
        "-nodelay",
    ): "set s [socket -server {} -myaddr 127.0.0.1 0]; set p [lindex [fconfigure $s -sockname] 2]; set c [socket 127.0.0.1 $p]; fconfigure $c -nodelay 1; close $c; close $s",
    ("fconfigure", None, "-inputmode"): "fconfigure stdin -inputmode normal",
    # ---- clock scan options ----
    ("clock", "scan", "-base"): "clock scan now -base 0",
    ("clock", "scan", "-format"): "clock scan {2020-01-01} -format {%Y-%m-%d}",
    ("clock", "scan", "-gmt"): "clock scan {2020-01-01} -gmt 1 -format {%Y-%m-%d}",
    ("clock", "scan", "-locale"): "clock scan {2020-01-01} -locale C -format {%Y-%m-%d}",
    ("clock", "scan", "-timezone"): "clock scan {2020-01-01} -timezone :UTC -format {%Y-%m-%d}",
    ("clock", "scan", "-validate"): "clock scan {2020-13-01} -validate 0 -format {%Y-%m-%d}",
    # ---- socket ----
    ("socket", None, "-async"): "set s [socket -async 127.0.0.1 1]; close $s",
    ("socket", None, "-myaddr"): "set s [socket -server {} -myaddr 127.0.0.1 0]; close $s",
    (
        "socket",
        None,
        "-myport",
    ): "set s [socket -server {} -myport 0 -myaddr 127.0.0.1 0]; close $s",
    ("socket", None, "-server"): "set s [socket -server {} -myaddr 127.0.0.1 0]; close $s",
    (
        "socket",
        None,
        "-reuseaddr",
    ): "set s [socket -server {} -reuseaddr 1 -myaddr 127.0.0.1 0]; close $s",
    (
        "socket",
        None,
        "-reuseport",
    ): "set s [socket -server {} -reuseport 1 -myaddr 127.0.0.1 0]; close $s",
    # ---- source ----
    (
        "source",
        None,
        "-encoding",
    ): "set f [open /tmp/_audit_src w]; close $f; source -encoding utf-8 /tmp/_audit_src",
    # ---- unset ----
    ("unset", None, "-nocomplain"): "unset -nocomplain ::nonexistent_var_42",
    # ---- string compare/equal ----
    ("string", "compare", "-nocase"): "string compare -nocase A a",
    ("string", "compare", "-length"): "string compare -length 1 ab ac",
    ("string", "equal", "-nocase"): "string equal -nocase A a",
    ("string", "equal", "-length"): "string equal -length 1 ab ac",
    ("string", "is", "-strict"): "string is integer -strict 123",
    ("string", "is", "-failindex"): "string is integer -failindex foo 12X",
    ("string", "map", "-nocase"): "string map -nocase {A B} aA",
    ("string", "match", "-nocase"): "string match -nocase A a",
    # ---- switch ----
    ("switch", None, "-exact"): "switch -exact a {a {set x 1}}",
    ("switch", None, "-glob"): "switch -glob a {a* {set x 1}}",
    ("switch", None, "-regexp"): "switch -regexp a {{^a$} {set x 1}}",
    ("switch", None, "-nocase"): "switch -nocase A {a {set x 1}}",
    ("switch", None, "-matchvar"): "switch -regexp -matchvar m a {{(.*)} {set x 1}}",
    # ---- subst ----
    ("subst", None, "-nobackslashes"): "subst -nobackslashes {\\n}",
    ("subst", None, "-nocommands"): "subst -nocommands {[set x]}",
    ("subst", None, "-novariables"): "subst -novariables {\\$x}",
    # ---- interp ----
    ("interp", "create", "-safe"): "set i [interp create -safe]; interp delete $i",
    ("interp", "cancel", "-unwind"): "set i [interp create]; interp cancel -unwind -- $i {}",
    (
        "interp",
        "invokehidden",
        "-global",
    ): "set i [interp create]; interp hide $i set; interp invokehidden $i -global set x 1; interp delete $i",
    (
        "interp",
        "invokehidden",
        "-namespace",
    ): "set i [interp create]; interp hide $i set; interp invokehidden $i -namespace :: set x 1; interp delete $i",
    # ---- package ----
    ("package", "present", "-exact"): "catch {package present -exact Tcl 9.0}",
    ("package", "require", "-exact"): "catch {package require -exact Tcl 9.0}",
    # ---- puts ----
    ("puts", None, "-nonewline"): "puts -nonewline {}",
    # ---- load ----
    ("load", None, "-global"): "catch {load -global /nonexistent}",
    ("load", None, "-lazy"): "catch {load -lazy /nonexistent}",
    # ---- unload ----
    ("unload", None, "-nocomplain"): "catch {unload -nocomplain /nonexistent}",
    ("unload", None, "-keeplibrary"): "catch {unload -keeplibrary /nonexistent}",
    # ---- encoding ----
    ("encoding", None, "-profile"): "catch {encoding convertfrom -profile strict utf-8 hi}",
    # ---- vwait (Tcl 9.0 added many) ----
    ("vwait", None, "-all"): "after 1 {set ::vw 1}; catch {vwait -all -timeout 100 -variable ::vw}",
    (
        "vwait",
        None,
        "-extended",
    ): "after 1 {set ::vw 1}; catch {vwait -extended -timeout 100 -variable ::vw}",
    (
        "vwait",
        None,
        "-nofileevents",
    ): "after 1 {set ::vw 1}; catch {vwait -nofileevents -timeout 100 -variable ::vw}",
    (
        "vwait",
        None,
        "-noidleevents",
    ): "after 1 {set ::vw 1}; catch {vwait -noidleevents -timeout 100 -variable ::vw}",
    (
        "vwait",
        None,
        "-notimerevents",
    ): "after 1 {set ::vw 1}; catch {vwait -notimerevents -timeout 100 -variable ::vw}",
    (
        "vwait",
        None,
        "-nowindowevents",
    ): "after 1 {set ::vw 1}; catch {vwait -nowindowevents -timeout 100 -variable ::vw}",
    ("vwait", None, "-readable"): "catch {vwait -readable stdin -timeout 1}",
    ("vwait", None, "-timeout"): "after 1 {set ::vw 1}; catch {vwait -timeout 100 -variable ::vw}",
    ("vwait", None, "-variable"): "after 1 {set ::vw 1}; vwait ::vw",
    ("vwait", None, "-writable"): "catch {vwait -writable stdout -timeout 1}",
}

# Errors that indicate the option was *not* recognised (vs. runtime failure).
NOT_SUPPORTED_TOKENS = (
    "bad option",
    "bad switch",  # Tcl 8.4 spelling for some commands.
    "unknown option",
    "unknown switch",
    "no such option",
    "invalid option",
    "unrecognized option",
    "ambiguous option",
    "ambiguous switch",
    "wrong # args:",
    "bad subcommand",  # subcommand-level option attached to wrong sub.
)


def probe(tclsh_dir: Path, script: str) -> tuple[bool, str]:
    """Run *script* under tclsh and return (ok, output).

    ``ok`` is False only if the output contains a "bad/unknown option" error
    or otherwise indicates the option was not recognised at parse/dispatch.
    Runtime errors (network, permission, channel) count as 'option recognised'.
    """
    tclsh = tclsh_dir / "tclsh"
    if not tclsh.exists():
        return (False, "tclsh not found")
    env = os.environ.copy()
    env["LD_LIBRARY_PATH"] = str(tclsh_dir)
    # Use catch so the script doesn't die on runtime errors.
    wrapped = f"if {{[catch {{{script}}} err]}} {{puts ERR:$err}} else {{puts OK}}"
    try:
        result = subprocess.run(
            [str(tclsh)],
            input=wrapped,
            text=True,
            capture_output=True,
            timeout=5,
            env=env,
        )
    except subprocess.TimeoutExpired:
        return (True, "timeout (option recognised, test ran)")
    output = (result.stdout + result.stderr).strip()
    for token in NOT_SUPPORTED_TOKENS:
        if token in output.lower():
            return (False, output)
    return (True, output)


def main() -> int:
    results: dict[tuple[str, str | None, str], dict[str, bool]] = defaultdict(dict)

    for key, script in PROBES.items():
        cmd, sub, opt = key
        label = f"{cmd}" + (f" {sub}" if sub else "") + f" {opt}"
        print(f"\n=== {label} ===")
        for ver, tcl_dir in TCL_DIRS.items():
            ok, out = probe(tcl_dir, script)
            mark = "✓" if ok else "✗"
            results[key][ver] = ok
            # Print short diagnostic on failure.
            extra = ""
            if not ok:
                first_line = out.split("\n")[0][:60]
                extra = f"  {first_line!r}"
            print(f"  {ver}: {mark}{extra}")

    # Save full results.
    out_path = ROOT / "tmp" / "option_dialect_audit.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    serialisable = [
        {
            "command": cmd,
            "subcommand": sub,
            "option": opt,
            "supported_in": [v for v, ok in versions.items() if ok],
        }
        for (cmd, sub, opt), versions in results.items()
    ]
    out_path.write_text(json.dumps(serialisable, indent=2) + "\n")
    print(f"\nFull results written to {out_path}")

    # Summary: which options need explicit dialect gating?
    print("\n=== Summary (options NOT universally available) ===")
    for entry in serialisable:
        all_versions = set(TCL_DIRS.keys())
        supported = set(entry["supported_in"])
        if supported != all_versions:
            label = (
                entry["command"]
                + (f" {entry['subcommand']}" if entry["subcommand"] else "")
                + f" {entry['option']}"
            )
            print(f"  {label:50s}  {sorted(supported)}")

    return 0


if __name__ == "__main__":
    sys.exit(main())

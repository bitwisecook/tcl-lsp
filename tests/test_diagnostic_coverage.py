"""Coverage enforcement for diagnostic/optimisation range accuracy.

Every registered code must be classified into exactly one bucket:

- ``FIXTURES`` — verified: the code fires on a trigger snippet covering the
  *exact, narrow* offending span, and does **not** fire on a clean snippet.
- ``RANGE_FIXME`` — the code fires (true positive) and is clean-clear (no
  false positive), but its range is still too wide / drops a trailing
  delimiter and needs narrowing.  Range is *not* asserted yet.
- ``NOT_YET_COVERED`` — no trigger fixture authored yet (often dialect- or
  context-specific).

The partition test fails if any code is unclassified or double-classified, so
a newly added code cannot slip through, and ``RANGE_FIXME`` / ``NOT_YET_COVERED``
only ever shrink as codes graduate into ``FIXTURES``.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import pytest

import core.common.codes_all  # noqa: F401 — registers every code
from core.common.codes import all_codes
from core.common.dialect import dialect_scope
from lsp.features.diagnostics import get_diagnostics


@dataclass(frozen=True)
class Case:
    source: str
    expected: str  # exact substring the range must cover
    clean: str  # a snippet on which the code must NOT fire
    dialect: str | None = None  # analyse under this dialect when set
    xc: bool = False  # enable XC translatability diagnostics
    contains: bool = False  # expected is a substring of a covering construct
    bigip: bool = False  # source is a BIG-IP .conf — use the bigip validator
    iapp: bool = False  # source is an APL presentation — use the iApp validator


@dataclass(frozen=True)
class FiresCase:
    source: str
    clean: str
    dialect: str | None = None
    bigip: bool = False
    iapp: bool = False


def _covered(source: str, r: Any) -> str:
    lines = source.split("\n")
    if r.start.line == r.end.line:
        return lines[r.start.line][r.start.character : r.end.character]
    return "\n".join(
        [
            lines[r.start.line][r.start.character :],
            *lines[r.start.line + 1 : r.end.line],
            lines[r.end.line][: r.end.character],
        ]
    )


def _run(
    source: str, dialect: str | None, xc: bool, bigip: bool = False, iapp: bool = False
) -> list[Any]:
    if bigip:
        from core.bigip.diagnostics import get_bigip_diagnostics
        from core.bigip.parser import parse_bigip_conf

        return get_bigip_diagnostics(parse_bigip_conf(source))
    if iapp:
        from core.bigip.apl_model import parse_apl
        from core.bigip.iapp_diagnostics import validate_iapp_presentation

        with dialect_scope("f5-iapps"):
            return validate_iapp_presentation(parse_apl(source))
    if dialect is not None:
        with dialect_scope(dialect):
            return get_diagnostics(source, xc_diagnostics_enabled=xc)
    return get_diagnostics(source, xc_diagnostics_enabled=xc)


def _matches(
    source: str,
    code: str,
    dialect: str | None = None,
    xc: bool = False,
    bigip: bool = False,
    iapp: bool = False,
) -> list[Any]:
    return [
        d
        for d in _run(source, dialect, xc, bigip, iapp)
        if (d.code if isinstance(d.code, str) else str(d.code)) == code
    ]


# ── verified: exact narrow range + no false positive ──────────────────

FIXTURES: dict[str, Case] = {
    "E001": Case("string\n", "string", "string length x\n"),
    "E002": Case("set\n", "set", "set x 1\n"),
    "E003": Case("string length a b c\n", "string", "string length a\n"),
    "W001": Case("string bogus x\n", "bogus", "string length x\n"),
    "W114": Case(
        "set x [expr {[expr {1}]}]\nputs $x\n", "[expr {1}]", "set x [expr {1}]\nputs $x\n"
    ),
    "W123": Case("boguscommand foo bar\n", "boguscommand", "puts hi\n"),
    "W212": Case("set $x 1\n", "$x", "set x 1\n"),
    "W300": Case("source $f\n", "$f", "source data.tcl\n"),
    "W230": Case("puts [lindex {a b} 5]\n", "5", "puts [lindex {a b} 1]\n"),
    "W232": Case("puts [string index abc 10]\n", "10", "puts [string index abc 1]\n"),
    "W240": Case("while {0} {puts x}\n", "{0}", "while {$go} {puts x}\n"),
    "W241": Case("while {1} {puts x}\n", "{1}", "while {$go} {set go 0}\n"),
    "W242": Case("while {$i < 10} {puts $i}\n", "{$i < 10}", "while {$i < 10} {incr i}\n"),
    "W307": Case("$cmd arg\n", "$cmd", "puts arg\n"),
    "W100": Case("if $x == 1 {puts hi}\n", "$x", "if {$x == 1} {puts hi}\n"),
    "W101": Case("eval $userinput\n", "$userinput", "eval {puts hi}\n"),
    "W102": Case("subst $x\n", "$x", "subst {literal}\n"),
    "W110": Case(
        'if {$x == "hello"} {set x done}\n',
        '{$x == "hello"}',
        'if {$x eq "hello"} {set x done}\n',
    ),
    "W112": Case("set x 1   \n", "   ", "set x 1\n"),
    "W201": Case(
        'set p "$dir/$file"\nputs $p\n', '"$dir/$file"', "set p [file join $d $f]\nputs $p\n"
    ),
    "W302": Case("catch {error oops}\n", "catch", "catch {error oops} result\n"),
    "W309": Case("eval [subst $x]\n", "[subst $x]", "eval {puts hi}\n"),
    "W312": Case("interp eval $i $code\n", "$code", "interp eval $i {puts hi}\n"),
    "W210": Case("puts $undefined\n", "$undefined", "set u 1\nputs $u\n"),
    "W211": Case("set unused 5\n", "unused", "set y 5\nputs $y\n"),
    "W213": Case("unset maybe\n", "maybe", "set m 1\nunset m\n"),
    "W220": Case("set dead 5\n", "dead", "set y 5\nputs $y\n"),
    "O100": Case("set x [expr {1 + 2}]\nputs $x\n", "$x", "puts hi\n"),
    "O102": Case("puts [expr {1 + 1}]\n", "[expr {1 + 1}]", "puts hi\n"),
    "O111": Case("expr $a + $b\n", "$a + $b", "expr {$a + $b}\n"),
    "O116": Case(
        "set x [list]\nlappend x a\nputs $x\n", "[list]", "set x {}\nlappend x a\nputs $x\n"
    ),
    "O118": Case("puts [lindex {a b c} 1]\n", "[lindex {a b c} 1]", "puts hi\n"),
    "O120": Case(
        'if {$x == "hello"} {set x done}\n', '{$x == "hello"}', 'if {$x eq "hello"} {set x done}\n'
    ),
    "IRULE1002": Case(
        "when BOGUS {\n}\n",
        "BOGUS",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
    ),
    "IRULE1004": Case(
        "when CLIENT_ACCEPTED {\n  log local0. hi\n}\n",
        "when",
        "when CLIENT_ACCEPTED priority 500 {\n  log local0. hi\n}\n",
        dialect="f5-irules",
    ),
    "IRULE2001": Case(
        "when HTTP_REQUEST {\n  matchclass $x equals $y\n}\n",
        "matchclass",
        "when HTTP_REQUEST {\n  class match $x equals $y\n}\n",
        dialect="f5-irules",
    ),
    "T100": Case(
        "when HTTP_REQUEST {\n  set u [HTTP::uri]\n  eval $u\n}\n",
        "$u",
        "when HTTP_REQUEST {\n  eval {puts hi}\n}\n",
        dialect="f5-irules",
    ),
    "T101": Case(
        "when HTTP_REQUEST {\n  set u [HTTP::uri]\n  puts $u\n}\n",
        "$u",
        "when HTTP_REQUEST {\n  puts hi\n}\n",
        dialect="f5-irules",
    ),
    "IRULE3001": Case(
        "when HTTP_REQUEST {\n  set u [HTTP::uri]\n  HTTP::respond 200 content $u\n}\n",
        "$u",
        "when HTTP_REQUEST {\n  HTTP::respond 200 content static\n}\n",
        dialect="f5-irules",
    ),
    "IRULE3002": Case(
        "when HTTP_REQUEST {\n  set u [HTTP::uri]\n  HTTP::header insert X $u\n}\n",
        "$u",
        "when HTTP_REQUEST {\n  HTTP::header insert X static\n}\n",
        dialect="f5-irules",
    ),
    # XC translatability classifications (need the xc flag).  The range covers
    # the classified construct, which is the context the user needs.
    "XC100": Case(
        "when HTTP_REQUEST { pool web_pool }\n",
        "pool web_pool",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
    ),
    "XC101": Case(
        "when HTTP_REQUEST { HTTP::redirect http://x }\n",
        "HTTP::redirect http://x",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
    ),
    "XC102": Case(
        'when HTTP_REQUEST { if {[HTTP::host] eq "x.com"} { pool p } }\n',
        'if {[HTTP::host] eq "x.com"}',
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
        contains=True,
    ),
    "XC103": Case(
        "when HTTP_REQUEST { HTTP::header insert X 1 }\n",
        "HTTP::header insert X 1",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
    ),
    "XC105": Case(
        "when HTTP_REQUEST { class match [HTTP::uri] eq dg }\n",
        "class match [HTTP::uri] eq dg",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
    ),
    "XC106": Case(
        "when HTTP_REQUEST { ASM::disable }\n",
        "ASM::disable",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
    ),
    "XC107": Case(
        "when HTTP_REQUEST { ASM::enable }\n",
        "ASM::enable",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
    ),
    "XC201": Case(
        "when HTTP_REQUEST_DATA { HTTP::payload }\n",
        "when HTTP_REQUEST_DATA",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
        contains=True,
    ),
    "XC203": Case(
        "when HTTP_REQUEST { if {$x} { pool p } }\n",
        "if {$x}",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
        contains=True,
    ),
    "XC250": Case(
        "when CLIENTSSL_HANDSHAKE { log local0. hi }\n",
        "when CLIENTSSL_HANDSHAKE",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
        contains=True,
    ),
    "XC300": Case(
        "when HTTP_REQUEST { eval $cmd }\n",
        "eval $cmd",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
    ),
    "XC301": Case(
        "when HTTP_REQUEST { TCP::collect }\n",
        "TCP::collect",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        xc=True,
    ),
    "W103": Case(
        'set f [open "| rm -rf /"]\n',
        "| rm -rf /",
        "set f [open data.txt]\n",
        contains=True,
    ),
    "W105": Case(
        'if {1} "puts $x"\n',
        '"puts $x"',
        "if {1} {puts $x}\n",
    ),
    "W113": Case(
        "proc set {a b} {return}\n",
        "set",
        "proc myproc {} {return}\n",
    ),
    "W115": Case(
        "# comment \\\nputs hi\n",
        "# comment \\\nputs hi",
        "# comment\nputs hi\n",
    ),
    "W121": Case(
        "set m 255.255.0.255\n",
        "255.255.0.255",
        "set m 255.255.255.0\n",
    ),
    "W125": Case(
        "if {1} {puts a}\nelse {puts b}\n",
        "else",
        "if {1} {puts a} else {puts b}\n",
    ),
    "W231": Case(
        "set l {a b}\nlset l 5 x\n",
        "5",
        "set l {a b}\nlset l 1 x\n",
    ),
    "W301": Case(
        'uplevel "set $x 1"\n',
        "set $x 1",
        "uplevel {set x 1}\n",
        contains=True,
    ),
    "W303": Case(
        "regexp {(a+)+$} $s\n",
        "(a+)+$",
        "regexp {abc} $s\n",
        contains=True,
    ),
    "W308": Case(
        "subst {$x [cmd]}\n",
        "subst",
        "subst -nocommands {$x}\n",
    ),
    "W313": Case(
        "file delete $path\n",
        "$path",
        "file delete /tmp/static\n",
    ),
    "H300": Case(
        "set x 1\nset x 1\n",
        "set x 1",
        "set x 1\nset y 2\n",
    ),
    "E102": Case(
        "set x }\n",
        "}",
        "set x y\n",
    ),
    "E201": Case(
        "set x [\n",
        "[",
        "set x [expr 1]\n",
    ),
    "O110": Case(
        "puts [expr {$x + 0}]\n",
        "[expr {$x + 0}]",
        "puts $x\n",
    ),
    "O114": Case(
        "set x [expr {$x + 1}]\nputs $x\n",
        "set x [expr {$x + 1}]",
        "incr x\nputs $x\n",
    ),
    "O117": Case(
        "if {[string length $s] == 0} {puts empty}\n",
        "{[string length $s] == 0}",
        'if {$s eq ""} {puts empty}\n',
    ),
    "O128": Case(
        "set L {a b c}\nputs [lindex $L [expr {[llength $L] - 1}]]\n",
        "[expr {[llength $L] - 1}]",
        "set L {a b c}\nputs [lindex $L 0]\n",
    ),
    "O124": Case(
        "when HTTP_REQUEST {\n  log local0. hi\n}\nproc unused {} {return 1}\n",
        "proc unused",
        "when HTTP_REQUEST {\n  call used\n}\nproc used {} {return 1}\n",
        dialect="f5-irules",
        contains=True,
    ),
    "O122": Case(
        "proc f {n} {\n  if {$n <= 0} {return 0}\n  return [f [expr {$n-1}]]\n}\n",
        "proc f",
        "proc f {n} {\n  return [expr {$n+1}]\n}\n",
        contains=True,
    ),
    "IRULE1005": Case(
        "when HTTP_REQUEST_DATA {\n  log local0. hi\n}\n",
        "HTTP_REQUEST_DATA",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
    ),
    "IRULE1006": Case(
        "when HTTP_REQUEST {\n  set p [HTTP::payload]\n}\n",
        "HTTP::payload",
        "when HTTP_REQUEST {\n}\n",
        dialect="f5-irules",
        contains=True,
    ),
    "IRULE1201": Case(
        "when HTTP_REQUEST {\n  HTTP::respond 200\n  HTTP::header insert X 1\n}\n",
        "HTTP::header insert X 1",
        "when HTTP_REQUEST {\n  HTTP::respond 200\n}\n",
        dialect="f5-irules",
    ),
    "IRULE3003": Case(
        "when HTTP_REQUEST {\n  set u [HTTP::uri]\n  log local0. $u\n}\n",
        "$u",
        "when HTTP_REQUEST {\n  log local0. hi\n}\n",
        dialect="f5-irules",
    ),
    "IRULE3102": Case(
        "when HTTP_REQUEST {\n  set u [HTTP::uri]\n}\n",
        "HTTP::uri",
        "when HTTP_REQUEST {\n  set u [HTTP::uri -normalized]\n}\n",
        dialect="f5-irules",
    ),
    "IRULE4001": Case(
        "when HTTP_REQUEST {\n  set static::count 1\n}\n",
        "set",
        "when RULE_INIT {\n  set static::count 1\n}\n",
        dialect="f5-irules",
    ),
    "IRULE5001": Case(
        "when HTTP_REQUEST {\n  log local0. hi\n}\n",
        "log",
        "when RULE_INIT {\n  log local0. hi\n}\n",
        dialect="f5-irules",
    ),
    "IRULE5005": Case(
        "proc helper {} {return 1}\nwhen HTTP_REQUEST {\n  helper\n}\n",
        "helper",
        "proc helper {} {return 1}\nwhen HTTP_REQUEST {\n  call helper\n}\n",
        dialect="f5-irules",
    ),
    "IRULE5007": Case(
        "HTTP::uri\n",
        "HTTP::uri",
        "set x 1\n",
        dialect="f5-irules",
    ),
    "IRULE2101": Case(
        "when HTTP_REQUEST {\n  if {[regexp {^/api/.*} $s]} {pool p}\n}\n",
        "regexp",
        "when HTTP_REQUEST {\n  log local0. hi\n}\n",
        dialect="f5-irules",
    ),
    "IRULE5006": Case(
        "when HTTP_REQUEST {\n  if {1} {\n    when HTTP_RESPONSE {log local0. x}\n  }\n}\n",
        "when",
        "when HTTP_REQUEST {\n  log local0. hi\n}\n",
        dialect="f5-irules",
    ),
    "IRULE6001": Case(
        "when HTTP_REQUEST {\n  global g\n}\n",
        "global",
        "when HTTP_REQUEST {\n  log local0. hi\n}\n",
        dialect="f5-irules",
    ),
    "IRULE3101": Case(
        "when HTTP_REQUEST {\n  HTTP::uri foo\n}\n",
        "HTTP::uri foo",
        "when HTTP_REQUEST {\n  HTTP::uri /foo\n}\n",
        dialect="f5-irules",
    ),
    "IRULE1001": Case(
        "when CLIENT_ACCEPTED {\n  HTTP::uri\n}\n",
        "HTTP::uri",
        "when CLIENT_ACCEPTED {\n  log local0. hi\n}\n",
        dialect="f5-irules",
    ),
    "IRULE1007": Case(
        "when CLIENT_ACCEPTED {\n  TCP::collect\n}\n",
        "TCP::collect",
        "when CLIENT_ACCEPTED {\n  TCP::collect\n  TCP::release\n}\n",
        dialect="f5-irules",
    ),
    "W002": Case(
        "when HTTP_REQUEST {\n  exec ls\n}\n",
        "exec",
        "when HTTP_REQUEST {\n  log local0. hi\n}\n",
        dialect="f5-irules",
    ),
    "W104": Case(
        'set l ""\nappend l " $x"\n',
        '" $x"',
        "set l {}\nlappend l $x\n",
    ),
    "W106": Case(
        'switch $x "a" "puts 1"\n',
        "puts 1",
        "switch -- $x a {puts 1}\n",
        contains=True,
    ),
    "W216": Case(
        "set ${arr}(x) 1\n",
        "${arr}(x)",
        "set arr(x) 1\n",
    ),
    "W304": Case(
        "switch $x a b\n",
        "$x",
        "switch -- $x a b\n",
    ),
    "O112": Case(
        "if {0} {\n  puts never\n}\n",
        "if {0} {\n  puts never\n}",
        "if {$x} {\n  puts maybe\n}\n",
    ),
    "O126": Case(
        "proc f {} {\n  set x 1\n  return 0\n}\n",
        "set x 1",
        "proc f {} {\n  set x 1\n  return $x\n}\n",
    ),
    "O105": Case(
        "set a [expr {$x + 1}]\nset b [expr {$x + 1}]\nputs $a$b\n",
        "expr {$x + 1}",
        "set a [expr {$x + 1}]\nputs $a\n",
        contains=True,
    ),
    "TK1001": Case(
        "package require Tk\nbutton .b\nlabel .l\npack .b\ngrid .l\n",
        "grid",
        "package require Tk\nbutton .b\npack .b\n",
    ),
    "TK1002": Case(
        "package require Tk\nbutton .frame.b\n",
        "button",
        "package require Tk\nframe .frame\nbutton .frame.b\n",
    ),
    "W003": Case(
        "expr {2 in {1 2 3}}\n",
        "2 in",
        "expr {2 + 3}\n",
        dialect="tcl8.4",
        contains=True,
    ),
    "W004": Case(
        "lsort -stride 2 $l\n",
        "-stride",
        "lsort $l\n",
        dialect="tcl8.4",
    ),
    "W111": Case(
        'set x "' + "a" * 200 + '"\n',
        'set x "' + "a" * 200 + '"',
        "set x 1\n",
    ),
    "O107": Case(
        "while {1} {break}\nputs after\nset z 1\n",
        "set z 1",
        "puts hi\nputs there\n",
    ),
    "BIGIP6008": Case(
        "ltm pool /Common/p { }\n"
        "ltm virtual /Common/vs1 { destination /Common/1.1.1.1:80 pool /Common/p }\n",
        "{ }",
        "ltm pool /Common/p { members { /Common/n:80 { } } }\n"
        "ltm virtual /Common/vs1 { destination /Common/1.1.1.1:80 pool /Common/p }\n",
        bigip=True,
    ),
}

# ── fires + clean-clear, but range still too wide (narrowing pending) ──

# Empty: every previously-too-wide range has been narrowed and graduated into
# FIXTURES.  New too-wide-but-firing codes can be parked here while pending.
RANGE_FIXME: dict[str, FiresCase] = {
    # File-level inconsistent line endings: range points at the first
    # character of the file rather than the offending line ending.
    "W118": FiresCase("set a 1\r\nset b 2\n", "set a 1\nset b 2\n"),
    # Invalid IP literal: range spans the whole `set` command rather than
    # the IP literal token.
    "W124": FiresCase("set ip 999.999.999.999\n", "set ip 1.2.3.4\n"),
    # String-build chain: each step's range bleeds onto the next line's
    # first character instead of ending at the statement.
    "O104": FiresCase("set s a\nappend s b\nappend s c\nputs $s\n", "set s abc\nputs $s\n"),
    # Dead store: range bleeds onto the following line's first character.
    "O109": FiresCase("set x 1\nset x 2\nputs $x\n", "set x 1\nputs $x\n"),
    # `drop` without `event disable all`/`return`: range points at the
    # event block's closing brace rather than the drop command.
    "IRULE5002": FiresCase(
        "when CLIENT_ACCEPTED {\n  drop\n}\n",
        "when CLIENT_ACCEPTED {\n  drop\n  return\n}\n",
        dialect="f5-irules",
    ),
    # `DNS::return` without `return`: range points at the event block's
    # closing brace rather than the DNS::return command.
    "IRULE5004": FiresCase(
        "when DNS_REQUEST {\n  DNS::return\n}\n",
        "when DNS_REQUEST {\n  DNS::return\n  return\n}\n",
        dialect="f5-irules",
    ),
    # Unused proc parameter: range spans the whole proc rather than the
    # offending parameter token.
    "W214": FiresCase("proc f {a b} {return $a}\n", "proc f {a} {return $a}\n"),
    # Shimmer inside loop body: range points at the pre-loop initialiser
    # rather than the in-loop conversion site.
    "S101": FiresCase(
        'set x 0\nwhile {1} {\n  set x [expr {$x + 1}]\n  set x "str"\n}\n',
        "set x 0\nputs $x\n",
    ),
    # Type oscillation across iterations: range drops the trailing quote.
    "S102": FiresCase(
        'set x 0\nwhile {1} {\n  set x [expr {$x + 1}]\n  set x "str"\n}\n',
        "set x 0\nwhile {$x < 3} {\n  incr x\n}\n",
    ),
    # Unknown widget option: range points at the widget command rather than
    # the offending `-option` token.
    "TK1003": FiresCase(
        "package require Tk\nbutton .b -bogusopt 1\n",
        "package require Tk\nbutton .b -text hi\n",
    ),
    # iRule references a pool that does not exist: range is empty (zero-width)
    # and needs to be anchored on the pool reference token.
    "BIGIP6002": FiresCase(
        "ltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/missing_pool }\n}\n",
        "ltm pool /Common/p { members { /Common/n:80 { } } }\n"
        "ltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/p }\n}\n",
        bigip=True,
    ),
    # Virtual references a missing iRule: range spans the whole virtual stanza
    # body rather than the offending rule reference.
    "BIGIP6003": FiresCase(
        "ltm virtual /Common/vs { destination /Common/1.1.1.1:80 rules { /Common/missing_rule } }\n",
        "ltm rule /Common/r {\nwhen HTTP_REQUEST { log local0. hi }\n}\n"
        "ltm virtual /Common/vs { destination /Common/1.1.1.1:80 rules { /Common/r } }\n",
        bigip=True,
    ),
    # Virtual references a missing pool: range spans the whole virtual stanza
    # body rather than the pool reference token.
    "BIGIP6005": FiresCase(
        "ltm virtual /Common/vs1 { destination /Common/1.1.1.1:80 pool /Common/missing }\n",
        "ltm pool /Common/p { members { /Common/n:80 { } } }\n"
        "ltm virtual /Common/vs1 { destination /Common/1.1.1.1:80 pool /Common/p }\n",
        bigip=True,
    ),
    # APL #include not found: AplInclude only tracks a line number, so the
    # range is a zero-width placeholder at column 0 pending a real span.
    "IAPP7003": FiresCase(
        '#include "/missing/file.tcl"\n',
        "set x 1\n",
        iapp=True,
    ),
}

# ── no trigger fixture yet (dialect/context-specific) ─────────────────
# This list only shrinks: as a code graduates into FIXTURES/RANGE_FIXME it
# must be removed here or the partition test fails.

NOT_YET_COVERED: frozenset[str] = frozenset(
    {
        "BIGIP6001",
        "BIGIP6004",
        "BIGIP6006",
        "BIGIP6007",
        "BIGIP6009",
        "BIGIP6010",
        "BIGIP6011",
        "E004",
        "E100",
        "E101",
        "E103",
        "E200",
        "E202",
        "E203",
        "IAPP7001",
        "IAPP7002",
        "IRULE1003",
        "IRULE1008",
        "IRULE1202",
        "IRULE2002",
        "IRULE2003",
        "IRULE3103",
        "IRULE4002",
        "IRULE4003",
        "IRULE4004",
        "IRULE4005",
        "IRULE5003",
        "O101",
        "O103",
        "O106",
        "O108",
        "O113",
        "O115",
        "O119",
        "O121",
        "O123",
        "O125",
        "O127",
        "S100",
        "T102",
        "T103",
        "T106",
        "W108",
        "W116",
        "W117",
        "W120",
        "W122",
        "W126",
        "W130",
        "W131",
        "W132",
        "W133",
        "W134",
        "W200",
        "W215",
        "W306",
        "W310",
        "W311",
        "XC200",
    }
)


def test_every_code_is_classified_exactly_once():
    covered = set(FIXTURES) | set(RANGE_FIXME) | set(NOT_YET_COVERED)
    registered = set(all_codes())

    unclassified = registered - covered
    assert not unclassified, (
        f"{len(unclassified)} code(s) are not classified into FIXTURES, "
        f"RANGE_FIXME, or NOT_YET_COVERED: {sorted(unclassified)}"
    )
    stale = covered - registered
    assert not stale, f"classified codes that no longer exist: {sorted(stale)}"

    overlap = (
        (set(FIXTURES) & set(RANGE_FIXME))
        | (set(FIXTURES) & NOT_YET_COVERED)
        | (set(RANGE_FIXME) & NOT_YET_COVERED)
    )
    assert not overlap, f"codes classified in more than one bucket: {sorted(overlap)}"


@pytest.mark.parametrize("code", sorted(FIXTURES))
def test_fixture_fires_with_exact_range(code):
    case = FIXTURES[code]
    matches = _matches(case.source, code, case.dialect, case.xc, case.bigip, case.iapp)
    assert matches, f"{code} did not fire on {case.source!r}"
    covered = {_covered(case.source, d.range) for d in matches}
    if case.contains:
        assert any(case.expected in c for c in covered), (
            f"{code} should cover a span containing {case.expected!r}; covered {sorted(covered)}"
        )
    else:
        assert case.expected in covered, (
            f"{code} should cover {case.expected!r}; covered {sorted(covered)}"
        )


@pytest.mark.parametrize("code", sorted(FIXTURES))
def test_fixture_no_false_positive(code):
    case = FIXTURES[code]
    assert not _matches(case.clean, code, case.dialect, case.xc, case.bigip, case.iapp), (
        f"{code} should not fire on clean {case.clean!r}"
    )


@pytest.mark.parametrize("code", sorted(RANGE_FIXME))
def test_range_fixme_fires_and_is_clean(code):
    case = RANGE_FIXME[code]
    assert _matches(case.source, code, case.dialect, bigip=case.bigip, iapp=case.iapp), (
        f"{code} did not fire on {case.source!r}"
    )
    assert not _matches(case.clean, code, case.dialect, bigip=case.bigip, iapp=case.iapp), (
        f"{code} should not fire on clean {case.clean!r}"
    )

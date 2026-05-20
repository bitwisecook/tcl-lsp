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

import pytest
from lsprotocol import types

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


@dataclass(frozen=True)
class FiresCase:
    source: str
    clean: str
    dialect: str | None = None


def _covered(source: str, r: types.Range) -> str:
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


def _run(source: str, dialect: str | None, xc: bool) -> list[types.Diagnostic]:
    if dialect is not None:
        with dialect_scope(dialect):
            return get_diagnostics(source, xc_diagnostics_enabled=xc)
    return get_diagnostics(source, xc_diagnostics_enabled=xc)


def _matches(
    source: str, code: str, dialect: str | None = None, xc: bool = False
) -> list[types.Diagnostic]:
    return [
        d
        for d in _run(source, dialect, xc)
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
}

# ── fires + clean-clear, but range still too wide (narrowing pending) ──

# Empty: every previously-too-wide range has been narrowed and graduated into
# FIXTURES.  New too-wide-but-firing codes can be parked here while pending.
RANGE_FIXME: dict[str, FiresCase] = {}

# ── no trigger fixture yet (dialect/context-specific) ─────────────────
# This list only shrinks: as a code graduates into FIXTURES/RANGE_FIXME it
# must be removed here or the partition test fails.

NOT_YET_COVERED: frozenset[str] = frozenset(
    {
        "BIGIP6001",
        "BIGIP6002",
        "BIGIP6003",
        "BIGIP6004",
        "BIGIP6005",
        "BIGIP6006",
        "BIGIP6007",
        "BIGIP6008",
        "BIGIP6009",
        "BIGIP6010",
        "BIGIP6011",
        "E004",
        "E100",
        "E101",
        "E102",
        "E103",
        "E200",
        "E201",
        "E202",
        "E203",
        "H300",
        "IAPP7001",
        "IAPP7002",
        "IAPP7003",
        "IRULE1001",
        "IRULE1003",
        "IRULE1005",
        "IRULE1006",
        "IRULE1007",
        "IRULE1008",
        "IRULE1201",
        "IRULE1202",
        "IRULE2002",
        "IRULE2003",
        "IRULE2101",
        "IRULE3003",
        "IRULE3101",
        "IRULE3102",
        "IRULE3103",
        "IRULE4001",
        "IRULE4002",
        "IRULE4003",
        "IRULE4004",
        "IRULE4005",
        "IRULE5001",
        "IRULE5002",
        "IRULE5003",
        "IRULE5004",
        "IRULE5005",
        "IRULE5006",
        "IRULE5007",
        "IRULE6001",
        "O101",
        "O103",
        "O104",
        "O105",
        "O106",
        "O107",
        "O108",
        "O109",
        "O110",
        "O112",
        "O113",
        "O114",
        "O115",
        "O117",
        "O119",
        "O121",
        "O122",
        "O123",
        "O124",
        "O125",
        "O126",
        "O127",
        "O128",
        "S100",
        "S101",
        "S102",
        "T102",
        "T103",
        "T106",
        "TK1001",
        "TK1002",
        "TK1003",
        "W002",
        "W003",
        "W004",
        "W103",
        "W104",
        "W105",
        "W106",
        "W108",
        "W111",
        "W113",
        "W115",
        "W116",
        "W117",
        "W118",
        "W120",
        "W121",
        "W122",
        "W124",
        "W125",
        "W126",
        "W130",
        "W131",
        "W132",
        "W133",
        "W134",
        "W200",
        "W214",
        "W215",
        "W216",
        "W231",
        "W301",
        "W303",
        "W304",
        "W306",
        "W308",
        "W310",
        "W311",
        "W313",
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
    matches = _matches(case.source, code, case.dialect, case.xc)
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
    assert not _matches(case.clean, code, case.dialect, case.xc), (
        f"{code} should not fire on clean {case.clean!r}"
    )


@pytest.mark.parametrize("code", sorted(RANGE_FIXME))
def test_range_fixme_fires_and_is_clean(code):
    case = RANGE_FIXME[code]
    assert _matches(case.source, code, case.dialect), f"{code} did not fire on {case.source!r}"
    assert not _matches(case.clean, code, case.dialect), (
        f"{code} should not fire on clean {case.clean!r}"
    )

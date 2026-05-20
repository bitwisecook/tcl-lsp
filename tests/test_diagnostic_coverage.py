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
from lsp.features.diagnostics import get_diagnostics


@dataclass(frozen=True)
class Case:
    source: str
    expected: str  # exact substring the range must cover
    clean: str  # a snippet on which the code must NOT fire


@dataclass(frozen=True)
class FiresCase:
    source: str
    clean: str


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


def _matches(source: str, code: str) -> list[types.Diagnostic]:
    return [
        d
        for d in get_diagnostics(source)
        if (d.code if isinstance(d.code, str) else str(d.code)) == code
    ]


# ── verified: exact narrow range + no false positive ──────────────────

FIXTURES: dict[str, Case] = {
    "E002": Case("set\n", "set", "set x 1\n"),
    "W001": Case("string bogus x\n", "bogus", "string length x\n"),
    "W100": Case("if $x == 1 {puts hi}\n", "$x", "if {$x == 1} {puts hi}\n"),
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
    "W210": Case("puts $undefined\n", "$undefined", "set u 1\nputs $u\n"),
    "W211": Case("set unused 5\n", "unused", "set y 5\nputs $y\n"),
    "W213": Case("unset maybe\n", "maybe", "set m 1\nunset m\n"),
    "W220": Case("set dead 5\n", "dead", "set y 5\nputs $y\n"),
    "O100": Case("set x [expr {1 + 2}]\nputs $x\n", "$x", "puts hi\n"),
    "O102": Case("puts [expr {1 + 1}]\n", "[expr {1 + 1}]", "puts hi\n"),
    "O111": Case("expr $a + $b\n", "$a + $b", "expr {$a + $b}\n"),
    "O120": Case(
        'if {$x == "hello"} {set x done}\n', '{$x == "hello"}', 'if {$x eq "hello"} {set x done}\n'
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
        "E001",
        "E003",
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
        "IRULE1002",
        "IRULE1003",
        "IRULE1004",
        "IRULE1005",
        "IRULE1006",
        "IRULE1007",
        "IRULE1008",
        "IRULE1201",
        "IRULE1202",
        "IRULE2001",
        "IRULE2002",
        "IRULE2003",
        "IRULE2101",
        "IRULE3001",
        "IRULE3002",
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
        "O116",
        "O117",
        "O118",
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
        "T100",
        "T101",
        "T102",
        "T103",
        "T106",
        "TK1001",
        "TK1002",
        "TK1003",
        "W002",
        "W003",
        "W004",
        "W101",
        "W102",
        "W103",
        "W104",
        "W105",
        "W106",
        "W108",
        "W111",
        "W113",
        "W114",
        "W115",
        "W116",
        "W117",
        "W118",
        "W120",
        "W121",
        "W122",
        "W123",
        "W124",
        "W125",
        "W126",
        "W130",
        "W131",
        "W132",
        "W133",
        "W134",
        "W200",
        "W212",
        "W214",
        "W215",
        "W216",
        "W230",
        "W231",
        "W232",
        "W240",
        "W241",
        "W242",
        "W300",
        "W301",
        "W303",
        "W304",
        "W306",
        "W307",
        "W308",
        "W309",
        "W310",
        "W311",
        "W312",
        "W313",
        "XC100",
        "XC101",
        "XC102",
        "XC103",
        "XC105",
        "XC106",
        "XC107",
        "XC200",
        "XC201",
        "XC203",
        "XC250",
        "XC300",
        "XC301",
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
    matches = _matches(case.source, code)
    assert matches, f"{code} did not fire on {case.source!r}"
    covered = {_covered(case.source, d.range) for d in matches}
    assert case.expected in covered, (
        f"{code} should cover {case.expected!r}; covered {sorted(covered)}"
    )


@pytest.mark.parametrize("code", sorted(FIXTURES))
def test_fixture_no_false_positive(code):
    case = FIXTURES[code]
    assert not _matches(case.clean, code), f"{code} should not fire on clean {case.clean!r}"


@pytest.mark.parametrize("code", sorted(RANGE_FIXME))
def test_range_fixme_fires_and_is_clean(code):
    case = RANGE_FIXME[code]
    assert _matches(case.source, code), f"{code} did not fire on {case.source!r}"
    assert not _matches(case.clean, code), f"{code} should not fire on clean {case.clean!r}"

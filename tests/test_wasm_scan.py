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

"""WASM ``scan`` command contracts pinned against ``runtime/zig/cmds/scan.zig``.

* ``%[...]`` character-set conversion (``BuildCharSet`` / ``CharInSet``):
  membership, ``^`` negation, ``a-z`` ranges (order-normalised), a
  leading ``]`` as a literal member, and a ``-`` adjacent to ``]`` as a
  literal.  Regression guard for the ``scan-1.*`` cluster.
* ``%e`` / ``%f`` / ``%g`` boxing a real ``TYPE_FLOAT`` instead of the
  old integer truncation (``scan 0.2 %f`` used to return ``0``).
  Regression guard for ``scan-4.49`` / ``scan-11.*`` / ``scan-14.*``.
* Inline (no-vars) empty-fill: the result list has one element per
  conversion specifier, with exhausted / skipped slots filled by empty
  strings (and XPG ``%N$`` building a position-ordered list).  A total
  underflow before any conversion collapses to the empty list / ``-1``.
  Regression guard for ``scan-12.*`` / ``scan-13.*`` / ``scan-10.7`` /
  ``scan-4.14``-``4.17``.
* ``%u`` / ``%b`` recognised as integer specifiers (``scan-5.14`` /
  ``scan-5.16``) and XPG ``ValidateFormat`` error messages
  (``scan-13.3`` / ``scan-13.9`` / ``scan-3.*``).
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # %[...] character sets — mirrors scan-1.1 .. 1.8.
        ("puts [list [scan foo {%[^o]} x] $x]", "1 f"),
        ('set n [scan {]foo} {%[]f]} x]\nputs "$n $x"', "1 ]f"),
        ("puts [list [scan abc-def {%[a-c]} x] $x]", "1 abc"),
        ("puts [list [scan -abc-def {%[-ac]} x] $x]", "1 -a"),
        ("puts [list [scan -abc-def {%[ac-]} x] $x]", "1 -a"),
        ("puts [list [scan abc-def {%[c-a]} x] $x]", "1 abc"),
        ("puts [list [scan def-abc {%[^c-a]} x] $x]", "1 def-"),
        # charset following another conversion (scan-10.6).
        ("puts [scan 5a {%i%[a]}]", "5 a"),
        # Real float scanning — scan-4.49 / scan-11.1 / scan-14.1.
        ('scan {.1 0.2 3.} {%e %f %g} x y z\nputs "$x $y $z"', "0.1 0.2 3.0"),
        ("scan {123 13.6} {%s %f} a b\nputs $b", "13.6"),
        ("scan -2.5 %f q\nputs $q", "-2.5"),
        ("scan Inf %g d\nputs $d", "Inf"),
    ],
)
def test_scan_charset_and_float(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # XPG3 positional conversions (%N$) — scan-4.11 / scan-13.1.
        ('scan {   abc   def   } {%2$s %1$s} x y\nputs "$x $y"', "def abc"),
        ("puts [scan a {%1$c}]", "97"),
        ('scan {12 34} {%2$d %1$d} a b\nputs "$a $b"', "34 12"),
        # Width fields cap the characters consumed — scan-4.12.
        (
            'scan {abc123456789012} {%3s%3d%3f%3[0-9]%s} a b c d e\nputs "$a $b $c $d $e"',
            "abc 123 456.0 789 012",
        ),
        ("scan 123456 %3d x\nputs $x", "123"),
        ("scan 3.14159 %4f x\nputs $x", "3.14"),
        # Sequential + suppress still work alongside the refactor.
        ('scan {1 2 3} {%d %*d %d} a b\nputs "$a $b"', "1 3"),
        # Incomplete exponent: %f takes the valid prefix and leaves `e`
        # for the next conversion rather than failing the whole scan.
        ('scan 1e {%f%s} a b\nputs "$a $b"', "1.0 e"),
        ('scan 2.5e {%f%s} a b\nputs "$a $b"', "2.5 e"),
    ],
)
def test_scan_xpg_and_width(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # Inline (no-vars) form fills exhausted conversions with empty
        # strings rather than stopping early — scan-12.2 / 12.3 / 12.7.
        ("puts [scan abc %c%c%c%c]", "97 98 99 {}"),
        ("puts [scan abc %s%c]", "abc {}"),
        # A *leading* empty slot must survive as ``{}`` — scan-13.4.
        ("puts [scan abc {%2$s%1$c}]", "{} abc"),
        # A bare literal mismatch (not end-of-input) still fills every
        # declared slot with empties — scan-12.5.
        ("puts [scan abc bogus%c%c%c]", "{} {} {}"),
        # ...but running out of input before any conversion matched is a
        # total failure: empty list inline — scan-12.4 / 12.6.
        ("puts [list [scan abc abc%c]]", "{}"),
        ("puts [list [catch {scan foo foobar} m] $m]", "0 {}"),
        # A non-matching charset after a good conversion leaves an empty
        # trailing slot — scan-10.7.
        ("puts [scan {5 a} {%i%[a]}]", "5 {}"),
        # XPG inline builds a position-ordered list, gaps empty-filled —
        # scan-13.6 / 13.7-style.
        (
            "catch {scan abc {bogus%1$c%5$c%10$c}} m; puts [list [llength $m] $m]",
            "10 {{} {} {} {} {} {} {} {} {} {}}",
        ),
    ],
)
def test_scan_inline_empty_fill(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # Classic form: total failure on underflow returns -1, not 0 —
        # scan-4.14 / 4.15 / 4.17.
        ("set x foo\nputs [list [scan {a} {a%d} x] $x]", "-1 foo"),
        ("set x foo\nputs [list [scan {} {a%d} x] $x]", "-1 foo"),
        # A plain mismatch (no underflow) with vars returns 0, not -1.
        ("set x foo\nputs [scan abc bogus%d x]", "0"),
    ],
)
def test_scan_underflow_classic(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # %u / %b are now recognised specifiers — scan-5.14 / 5.16.
        # (Tcl's intricate 32-bit saturate/wrap on overflow is not
        # modelled; these inputs stay in range.)
        ("puts [scan 0xff %u]", "0"),
        ("puts [scan 0x40 %b]", "0"),
        ("puts [scan 1010 %b]", "10"),
        ("puts [scan 777 %o]", "511"),
        ("puts [scan ff %x]", "255"),
    ],
)
def test_scan_unsigned_bases(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # XPG validation errors mirror C ValidateFormat — scan-13.3 / 3.7.
        (
            "catch {scan abc {%1$c%1$c}} m; puts $m",
            'variable is assigned by multiple "%n$" conversion specifiers',
        ),
        # Index out of range guards against billion-slot allocation —
        # scan-13.9.
        (
            "catch {scan abc {%2147483648$s}} m; puts $m",
            '"%n$" argument index out of range',
        ),
        # Mixing %n$ and sequential specs — scan-3.1.
        (
            "catch {scan {} {%d%1$d} x} m; puts $m",
            'cannot mix "%" and "%n$" conversion specifiers',
        ),
        # Width on %c is illegal — scan-3.5.
        (
            "catch {scan {} {%10c} a} m; puts $m",
            "field width may not be specified in %c conversion",
        ),
        # An XPG gap in the *variable* form is an error: ``%2$d`` leaves
        # the first variable unassigned.
        (
            "catch {scan {1} {%2$d} a b} m; puts $m",
            "variable is not assigned by any conversion specifiers",
        ),
        # Surplus variables (more names than conversions) error the same
        # way in the variable form.
        (
            "catch {scan {1 2} {%d %d} a b c} m; puts $m",
            "variable is not assigned by any conversion specifiers",
        ),
    ],
)
def test_scan_validation_errors(source: str, expected: str) -> None:
    assert _run(source) == expected


def test_scan_xpg_gap_inline_is_allowed() -> None:
    # The inline (no-variable) form legitimately yields leading empty list
    # elements for an XPG gap — only the variable form errors.
    assert _run("puts [scan {1} {%2$d}]") == "{} 1"

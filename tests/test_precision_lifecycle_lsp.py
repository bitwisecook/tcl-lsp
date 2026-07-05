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

"""LSP-surface regression tests for the dead-store / unused precision fixes.

These drive ``server.features.diagnostics.get_diagnostics`` — the exact path the
LSP server publishes to editors — so the fixes are proven at the protocol
surface, not just the analyser core.

Every negative case below is a construct where the variable IS read (verified
against C tclsh semantics), so W210/W211/W214/W220 must NOT fire; every positive
control is a genuine dead store / unused that MUST still fire.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from server.features.diagnostics import get_diagnostics

_LIFECYCLE = {"W210", "W211", "W214", "W220"}


def _lifecycle_codes(source: str) -> list[str]:
    return [
        d.code for d in get_diagnostics(source) if isinstance(d.code, str) and d.code in _LIFECYCLE
    ]


# --- Negatives: variable IS read; nothing should fire -----------------------

NEGATIVES = {
    # Read hidden inside a command-substituted expr — vars live in expr {...}.
    "expr-sub incr amount": "proc f {} { set w 5; set i 0; incr i [expr {$w}]; return $i }",
    "expr-sub in return": "proc f {n} { set scale 3; return [expr {$n * $scale}] }",
    # Read is the return value (CFGReturn terminator).
    "return read": "proc f {} { set x 1; return $x }",
    "return read after loop": "proc f {d} { set s 0; dict for {k v} $d { incr s $v }; return $s }",
    # Write is observable via a trace callback (tclsh fires it on `set`).
    "traced write (8.5+)": "proc f {} { trace add variable x write cb; set x 1 }",
    "traced write (8.4)": "proc f {} { trace variable x w cb; set x 1 }",
    # eval's braced body runs in the current scope and reads x.
    "eval body read": "proc f {} { set x 1; eval {puts $x} }",
    "eval body expr read": "proc f {} { set x 1; eval {set y [expr {$x + 1}]} }",
    # `catch {...} msg opts` inside a command substitution writes msg/opts in
    # the current scope (verified vs tclsh: `set e [catch {error b} m o]`
    # leaves $m and $o set), so reading them afterward is not read-before-set.
    "catch-cmdsub msg/opts read": (
        "proc f {} { set e [catch {error boom} msg opts]; if {$e} { return -options $opts $msg } }"
    ),
    "catch-cmdsub msg only": "proc f {} { if {[catch {set x 1} msg]} { return $msg } }",
    # regexp/scan match vars written inside a command substitution are
    # likewise current-scope writes, not read-before-set.
    "regexp-cmdsub match var": (
        "proc f {s} { set ok [regexp {(\\w+)} $s whole sub]; return [list $ok $whole $sub] }"
    ),
    "scan-cmdsub literal var": (
        "proc f {s} { set n [scan $s {%d %d} a b]; return [list $n $a $b] }"
    ),
    # catch buried in an expr body writes tmp at run time (tclsh: the [catch]
    # substitution executes during expr evaluation), so the `|| $tmp` read is
    # not read-before-set.
    "expr-catch result var": (
        "proc f {} { set eof [expr {[catch {someCmd} tmp] || $tmp}]; return $eof }"
    ),
    # A static (braced) body under a *dynamically-named* namespace eval is
    # still a known script: the inner proc's params must be scoped to that
    # proc, not leak into the enclosing scope as read-before-set.  (tclsh
    # evaluates the body in whatever namespace the name resolves to; the
    # name being computed does not change the body's lexical structure.)
    "dynamic-ns proc param": (
        "namespace eval [info object namespace ::C] { proc bar {x} { return $x } }"
    ),
    "dynamic-ns multi param": (
        "namespace eval [namespace current]::sub { proc f {a b} { return [list $a $b] } }"
    ),
    # `upvar 1 $name var` aliases `var` to a caller variable named at run time;
    # writing `var` writes the caller's variable, so it is not a dead store /
    # unused — even though the *target* name is dynamic and unresolvable.
    "upvar dynamic-target alias write": (
        "proc setN {name val} { upvar 1 $name var; set var $val }"
    ),
    "upvar braced-dynamic-target alias write": (
        "proc setInt {*var val} { upvar 1 ${*var} var; set var $val }"
    ),
    # The namespace-name expression of a `namespace eval $ns {…}` is evaluated
    # in the current scope, so `ns` is read there and is not set-but-unused.
    "dynamic-ns name is read": (
        "proc f {conn} { set name [format ns%s $conn]; namespace eval $name { variable s } }"
    ),
    # A read inside a BODY-role script nested in a command substitution — most
    # commonly `[catch {... $x ...} e]` in an if/while condition or expr — is a
    # real current-scope read (tclsh runs the catch body in the caller frame).
    "catch-body read in if condition": (
        "proc f {sock} { if {[catch {close $sock} e]} { puts failed } }"
    ),
    "catch-body read in while condition": (
        "proc f {chan} { while {[catch {gets $chan line}]} { break } }"
    ),
    "catch-body read in expr assignment": (
        "proc f {sock} { set r [expr {[catch {close $sock} e]}]; return $r }"
    ),
    # A var read only inside an [expr {...}] command substitution is a real
    # use (expr substitutes it at eval time); its feeding `set` is not dead.
    # (Gap B: this read now flows into SSA uses, not just name-level recovery.)
    "expr-cmdsub read keeps def live": (
        "proc f {} { set w 1; set i 0; incr i [expr {$w}]; return $i }"
    ),
}


@pytest.mark.parametrize("label", list(NEGATIVES))
def test_no_false_lifecycle_diagnostic(label: str) -> None:
    codes = _lifecycle_codes(NEGATIVES[label])
    assert codes == [], f"{label!r}: unexpected {codes} (variable is read)"


# --- Positives: genuine dead store / unused; must still fire ----------------


def test_genuine_dead_store_still_fires() -> None:
    # x v1 is overwritten before any read.
    assert "W220" in _lifecycle_codes("proc f {} { set x 1; set x 2; return $x }")


def test_genuine_unused_still_fires() -> None:
    # x is never read (eval body does not reference it).
    assert "W211" in _lifecycle_codes("proc f {} { set x 1; eval {puts hi} }")


def test_untraced_unrelated_var_still_fires() -> None:
    # Tracing x must not suppress an unrelated dead y.
    assert "W220" in _lifecycle_codes("proc f {} { trace add variable x write cb; set y 1 }")


def test_catch_cmdsub_unrelated_read_still_fires() -> None:
    # The catch writes msg, but `other` is genuinely never set — W210 must
    # still fire for it (the command-sub exemption is name-scoped, not blanket).
    codes = _lifecycle_codes("proc f {} { set e [catch {set x 1} msg]; puts $other }")
    assert "W210" in codes


def test_dynamic_ns_body_proc_genuine_rbs_still_fires() -> None:
    # Inlining the static body under a dynamic namespace name must still
    # *analyse* the inner proc: a genuine read-before-set inside it fires.
    codes = _lifecycle_codes("namespace eval [set ns] { proc bar {} { puts $undef } }")
    assert "W210" in codes


def test_dynamic_ns_body_proc_unused_param_still_fires() -> None:
    # The inner proc is a real scope, so an unused parameter is still flagged.
    codes = _lifecycle_codes("namespace eval [set ns] { proc bar {unusedparam} { return 1 } }")
    assert "W214" in codes


_PKGINDEX_SRC = "package ifneeded foo 1.0 [list source [file join $dir foo.tcl]]\n"


def test_pkgindex_dir_not_read_before_set() -> None:
    # Tcl's package loader sets $dir before sourcing pkgIndex.tcl, so the
    # ambient $dir read must not be flagged read-before-set there.
    codes = [d.code for d in get_diagnostics(_PKGINDEX_SRC, uri="file:///pkg/pkgIndex.tcl")]
    assert "W210" not in codes


def test_pkgindex_dir_fires_in_other_files() -> None:
    # The exemption is gated on the pkgIndex.tcl filename — a $dir read in any
    # other file is still a genuine read-before-set.
    codes = [d.code for d in get_diagnostics(_PKGINDEX_SRC, uri="file:///pkg/other.tcl")]
    assert "W210" in codes

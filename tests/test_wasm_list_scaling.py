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

"""WASM list/dict build scaling — the Phase 1 list internal rep.

``tcl_cmd_lappend`` used to re-validate the *entire* accumulator with
``check_list_syntax`` (an O(N) scan) on every append, so building an
N-element list — ``lappend`` in a loop, ``lsearch -all``, ``dict map``
collecting a result — was O(N^2).  At 100k elements this blew past the
180 s watchdog (dict-24.24 / dict-24.25 timed out).

Phase 1 gives a lappend-built list a ``TYPE_LIST`` marker meaning "this
buffer is a canonical list by construction", so subsequent appends skip
the re-validation and the build is back to amortised O(N).  These tests
exercise sizes that finish in well under a second when linear but would
trip the wasmtime watchdog if the quadratic regressed.

The runtime ``timeout_s`` is the real assertion: a regression to O(N^2)
makes ``_run_wasm`` raise (watchdog interrupt) and the test fails.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(body: str, timeout_s: float = 30.0) -> str:
    _, stdout = _run_wasm(_compile_tcl(body), capture_stdout=True, timeout_s=timeout_s)
    return stdout.rstrip("\n")


def test_lappend_loop_is_linear() -> None:
    # Build a 50k-element list one lappend at a time; O(N^2) would not
    # finish under the watchdog.  Verify length + a sampled element.
    body = (
        "set x {}\n"
        "for {set i 0} {$i < 50000} {incr i} { lappend x $i }\n"
        "puts [llength $x]\n"
        "puts [lindex $x 49999]\n"
    )
    assert _run(body) == "50000\n49999"


def test_lsearch_all_is_linear() -> None:
    # lsearch -all accumulates every match via tcl_cmd_lappend — the
    # exact path that made dict-24.24's input build quadratic.
    body = "puts [llength [lsearch -all [lrepeat 100000 x] x]]\n"
    assert _run(body) == "100000"


def test_dict_map_huge_dict_terminates() -> None:
    # dict-24.24 verbatim: dict map over a 100k-element dict, summing
    # k*v.  Hung (>120 s) before the fix; ~1.7 s after.
    body = (
        "puts [tcl::mathop::+ {*}"
        "[dict map {k v} [lsearch -all [lrepeat 100000 x] x] "
        "{ expr { $k * $v } }]]\n"
    )
    assert _run(body) == "166666666600000"


def test_lappend_still_validates_malformed_list() -> None:
    # The marker must not weaken correctness: appending to a value whose
    # string rep is NOT a canonical list still raises (listobj-4.4).
    body = 'set x "a \\{b"\nputs [catch {lappend x c} m]\nputs $m\n'
    assert _run(body) == "1\nunmatched open brace in list"


@pytest.mark.parametrize(
    "body,expected",
    [
        # A lappend-built list whose single element is numeric must still
        # parse as a number (the TYPE_LIST marker is scalar-transparent).
        ("set x {}\nlappend x 5\nputs [expr {$x + 1}]", "6"),
        ("set x {}\nlappend x 5.5\nputs [expr {$x + 1}]", "6.5"),
        ("set x {}\nlappend x a b c\nputs [lindex $x 1]", "b"),
    ],
)
def test_lappend_marker_is_scalar_transparent(body: str, expected: str) -> None:
    assert _run(body) == expected


def test_foreach_over_large_list_is_linear() -> None:
    # The compiled foreach calls tcl_cmd_list_index(L, i) per element with
    # monotonically increasing i.  Without the forward cursor cache each
    # call re-derived element i from the string (O(i)), making the loop
    # O(N^2) — foreach {k v} over a 20k list was ~35 s.  Now O(N).
    body = "set s 0\nforeach {k v} [lseq 1 20000] { incr s [expr {$k * $v}] }\nputs $s\n"
    assert _run(body) == "1333433330000"


def test_forward_lindex_loop_is_linear() -> None:
    # The same cursor cache covers a forward ``lindex $L $i`` loop.
    body = (
        "set L [lseq 0 19999]\n"
        "set s 0\n"
        "for {set i 0} {$i < 20000} {incr i} { incr s [lindex $L $i] }\n"
        "puts $s\n"
    )
    assert _run(body) == str(sum(range(20000)))


@pytest.mark.parametrize(
    "body,expected",
    [
        # Cursor cache correctness: random / backward / repeated access and
        # nested loops must all return the right element.
        ("puts [lindex {a b c d e} 3]", "d"),
        ("puts [lindex {a b c d e} end]", "e"),
        ('set L {a b c d}\nputs "[lindex $L 3][lindex $L 0][lindex $L 2]"', "dac"),
        (
            "set o {}\nforeach a {1 2 3} {foreach b {x y} {append o $a$b}}\nputs $o",
            "1x1y2x2y3x3y",
        ),
    ],
)
def test_list_index_cache_correctness(body: str, expected: str) -> None:
    assert _run(body) == expected


def test_list_index_cache_inline_slab_reuse() -> None:
    # Regression: the cursor cache must invalidate on object *release*, keyed
    # on the handle — inline strings (<=8 bytes, cap==0) keep their buffer in
    # the obj header and never hit the external-buffer free path.  Repeatedly
    # building, indexing, and discarding a short list reuses the freed slab; a
    # stale (handle, ptr, len) hit would return an element from a prior list.
    body = (
        "set ok 1\n"
        "for {set i 0} {$i < 5000} {incr i} {\n"
        "  set L [list a b c]\n"
        '  if {[lindex $L 0] ne "a" || [lindex $L 2] ne "c"} { set ok 0; break }\n'
        "  set M [list x y z]\n"
        '  if {[lindex $M 0] ne "x" || [lindex $M 2] ne "z"} { set ok 0; break }\n'
        "}\n"
        "puts $ok\n"
    )
    assert _run(body) == "1"

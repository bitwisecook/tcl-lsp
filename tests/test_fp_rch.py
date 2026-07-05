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

"""TP/FP regression tests for the RCH (reachability) family.

Each test pairs to an ``FP-RCH-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

RCH family covers:

* O107 — code-after-loop / handler-body reachability where pre-fix the
  loop-exit / handler block was a CFG island and every statement looked
  unreachable.
* The W210 'read before set' fallout of unreachable handlers (no SSA
  inheritance).

Ground truth: real C tclsh 9.0.3.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics


def _codes(source: str, code: str):
    return [d for d in get_diagnostics(source) if d.code == code]


# FP-RCH-01 — while 1 { break }: break-after is reachable


FP_RCH_01_REPRO = """\
proc f {c} {
    # while 1 with a conditional `break` -> `puts after` IS reachable.
    while 1 { if {$c} break }
    puts after
}
"""


def test_FP_RCH_01_while1_break_after_reachable():
    """FP: a conditional break inside `while 1` makes the post-loop block
    reachable — `puts after` must NOT fire O107."""
    assert _codes(FP_RCH_01_REPRO, "O107") == []


def test_FP_RCH_01_for_true_break_reachable():
    """FP control: same fix applies to `for {…} true {…}` constant-true forms."""
    src = "proc f {c} { for {set i 0} true {incr i} { if {$c} break }\n puts after }"
    assert _codes(src, "O107") == []


def test_FP_RCH_01_nested_loop_break_reachable():
    """FP: nested loops with break each feed their own loop-exit edge to
    reachability — the outermost `puts after` is still reachable."""
    src = (
        "proc f {c} { while 1 { foreach x {1 2} { if {$c} break }\n if {$c} break }\n puts after }"
    )
    assert _codes(src, "O107") == []


# FP-RCH-02 — try handler body is reachable


FP_RCH_02_REPRO = """\
proc f {} {
    # `on error` handler body is reachable; no O107 on `set y 1`.
    try {
        set x [doThing]
    } on error {e opts} {
        set y 1
        puts $y
    }
}
"""


def test_FP_RCH_02_handler_body_reachable():
    """FP: try handler body must not be flagged unreachable (pre-fix it was
    a CFG island with no predecessor edge)."""
    assert _codes(FP_RCH_02_REPRO, "O107") == []


def test_FP_RCH_02_handler_var_not_unset():
    """FP control: the handler-bound var `e` is defined by the handler clause
    itself — must NOT be W210 read-before-set."""
    src = "proc f {} {\n    try {\n        risky\n    } on error {e} {\n        puts $e\n    }\n}"
    assert _codes(src, "W210") == []


# FP-RCH-03 — on ok inherits body-defined SSA versions


FP_RCH_03_REPRO = """\
proc f {} {
    # `on ok` runs after the body completes; $vdata IS defined.
    try {
        set vdata [getData]
    } on ok {} {
        return $vdata
    }
}
"""


def test_FP_RCH_03_on_ok_reads_body_var():
    """FP: `on ok` runs after the body completes normally, so the body's SSA
    versions feed the handler — `$vdata` is defined."""
    assert _codes(FP_RCH_03_REPRO, "W210") == []


def test_FP_RCH_03_on_ok_unset_var_still_fires():
    """TP control: a `$vneversetbeforetry` read in `on ok` IS read-before-set —
    proves the SSA-inheritance fix doesn't blanket-suppress W210 in handlers."""
    src = (
        "proc f {} {\n"
        "    try {\n"
        "        set vdata [getData]\n"
        "    } on ok {} {\n"
        "        return $vneversetbeforetry\n"
        "    }\n"
        "}\n"
    )
    # The reachability + RBS fix must NOT blanket-suppress a genuine
    # read-before-set inside the handler: `$vneversetbeforetry` is never set on
    # any path into the `on ok` body, so W210 still fires (tclsh: "can't read
    # ... no such variable").  This pins that the SSA-inheritance fix kept real
    # handler RBS reporting alive.
    codes = {d.code for d in get_diagnostics(src)}
    assert "W210" in codes, codes


# FP-RCH-04 — genuine infinite loop (no break) → O107 IS reported


FP_RCH_04_REPRO = """\
proc f {} {
    # No break / return -> `puts after` IS dead code.
    while 1 { puts x }
    puts after
}
"""


def test_FP_RCH_04_infinite_loop_dead_code_fires():
    """TP: a `while 1` body with no `break`/`return`/throw really does make
    the post-loop block unreachable — O107 must still fire."""
    assert _codes(FP_RCH_04_REPRO, "O107"), (
        "infinite loop with no break must fire O107 on the post-loop block"
    )

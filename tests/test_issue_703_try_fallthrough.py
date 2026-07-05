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

"""Regression test for GitHub issue #703 -- ``try`` handler ``-`` fallthrough.

Issue #703 reported a **false positive**: the body of a ``try`` ``on``/``trap``
handler may be a bare ``-`` to make the clause share the *next* handler's body
(the same fallthrough mechanism ``switch`` uses for pattern bodies).  The
extension wrongly treated every handler body as a script, so the solo ``-`` was
lowered to a zero-argument call of the ``-`` command and flagged E002
("Too few arguments for '-': expected at least 1, got 0").

The reporter's working example (``tools/findBadExternals.tcl`` on Tcl's ``main``
branch) is::

    try {
        switch $::tcl_platform(platform) {
            unix - macosx { exec nm --extern-only --defined-only $libtcl }
            windows       { exec dumpbin /exports $libtcl }
        }
    } on ok result - trap NONE result {
        ...
        return 0
    } on error msg {
        ...
        return 1
    }

The fix lives in :func:`compiler.lowering.IRLowerer._lower_try` (a ``-`` handler
body lowers to an empty :class:`~compiler.ir.IRScript` with
``IRTryHandler.fallthrough=True`` rather than a ``-`` command call) and in
``dialects/tcl/try_.py`` (the ``-`` body gets no ``ArgRole.BODY``).

Per the Tcl C source (``TclNRTryObjCmd`` in ``tclCmdMZ.c``), a body of ``-``
is recognised by string value, so the braced ``{-}`` form is *also* a
fallthrough -- both are asserted here.  This file asserts **both** sides so the
wiring can't silently regress:

* **fix holds** -- the fallthrough ``-`` (and ``{-}``) handler body raises no
  E002, and lowers to a fallthrough handler with an empty body; and
* **true positive still fires** -- a genuine zero-argument ``-`` *command* call
  inside a real handler/proc body is still flagged E002.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.ir import IRScript, IRTry
from compiler.lowering import lower_to_ir
from server.features.diagnostics import get_diagnostics


def _codes(source: str, code: str):
    """Return diagnostics for *source* with the given code."""
    return [d for d in get_diagnostics(source) if d.code == code]


# The reporter's exact snippet (header comments omitted, as in the issue).
_ISSUE_SOURCE = """\
proc main {argc argv} {
    if {$argc != 1} {
        puts stderr "syntax is: [info script] libtcl"
        return 1
    }
    lassign $argv libtcl

    try {
        switch $::tcl_platform(platform) {
            unix - macosx {
                exec nm --extern-only --defined-only $libtcl
            }
            windows {
                exec dumpbin /exports $libtcl
            }
        }
    } on ok result - trap NONE result {
        foreach line [split $result \\n] {
            if {! [string match {* [Tt]cl*} $line]} {
                puts $line
            }
        }
        return 0
    } on error msg {
        puts stderr $msg
        return 1
    }
}
exit [main $::argc $::argv]
"""


def _e002(source: str):
    """Return all E002 diagnostics for *source*."""
    return [d for d in get_diagnostics(source) if d.code == "E002"]


def _find_try(script: IRScript) -> IRTry | None:
    """Depth-first search for the first :class:`IRTry` in *script*."""
    for stmt in script.statements:
        if isinstance(stmt, IRTry):
            return stmt
        for attr in ("body", "then_body", "else_body", "finally_body"):
            sub = getattr(stmt, attr, None)
            if isinstance(sub, IRScript):
                found = _find_try(sub)
                if found is not None:
                    return found
    return None


class TestIssue703TryFallthrough:
    """#703 -- ``try`` handler body of ``-`` is a fallthrough, not a command."""

    def test_fix_holds_reporter_snippet_has_no_e002(self):
        # The exact reporter snippet: the solo ``-`` on the
        # ``on ok result - trap NONE result`` line must not be flagged.
        assert _e002(_ISSUE_SOURCE) == []

    def test_fallthrough_handler_lowers_to_empty_body(self):
        ir = lower_to_ir(_ISSUE_SOURCE)
        stmt = _find_try(ir.procedures["::main"].body)
        assert stmt is not None, "expected an IRTry in proc main"
        kinds = [(h.kind, h.match_arg, h.fallthrough) for h in stmt.handlers]
        assert kinds == [
            ("on", "ok", True),
            ("trap", "NONE", False),
            ("on", "error", False),
        ]
        # The fallthrough handler carries no statements of its own; the
        # shared code lives in the following ``trap NONE`` handler.
        fall, target = stmt.handlers[0], stmt.handlers[1]
        assert fall.body.statements == ()
        assert len(target.body.statements) >= 1

    def test_braced_dash_body_is_also_fallthrough(self):
        # Per Tcl semantics a body of ``{-}`` evaluates to the string
        # ``-`` and is equally a fallthrough marker.
        source = """\
proc p {} {
    try {
        set x 1
    } on ok a {-} trap NONE b {
        return $b
    }
}
"""
        assert _e002(source) == []
        ir = lower_to_ir(source)
        stmt = _find_try(ir.procedures["::p"].body)
        assert stmt is not None
        assert stmt.handlers[0].fallthrough is True
        assert stmt.handlers[0].body.statements == ()

    def test_consecutive_fallthrough_handlers(self):
        # Several ``-`` handlers in a row all share the final body.
        source = """\
proc p {} {
    try {
        set x 1
    } on ok a - on return b - trap NONE c {
        return $c
    }
}
"""
        assert _e002(source) == []
        ir = lower_to_ir(source)
        stmt = _find_try(ir.procedures["::p"].body)
        assert stmt is not None
        flags = [h.fallthrough for h in stmt.handlers]
        assert flags == [True, True, False]

    def test_genuine_dash_command_in_handler_body_still_flagged(self):
        # A real zero-arg ``-`` *command* inside a handler body is still a
        # genuine arity error -- the fix must not blunt E002 here.
        source = """\
proc p {} {
    try {
        set x 1
    } on error msg {
        -
    }
}
"""
        assert len(_e002(source)) == 1

    def test_genuine_dash_command_outside_try_still_flagged(self):
        # Bare ``-`` as a command in a plain proc body stays an arity error.
        assert len(_e002("proc f {} {\n    -\n}\n")) == 1

    def test_no_false_w210_on_fallthrough_group_var(self):
        # Codex review follow-up: when a fallthrough handler's var name
        # differs from the target's, the shared body must not be flagged
        # W210 for reading either var.  Real Tcl is itself inconsistent
        # here — a byte-compiled (proc) ``try`` binds the *matching*
        # handler's var, while the interpreted form binds the *target*'s —
        # so the shared body is analysed with the whole group's vars
        # treated as defined.
        reads_fallthrough_var = """\
proc p {} {
    try {set v ok} on ok x - on error y {
        return $x
    }
}
"""
        reads_target_var = """\
proc p {} {
    try {set v ok} on ok x - on error y {
        return $y
    }
}
"""
        assert _codes(reads_fallthrough_var, "W210") == []
        assert _codes(reads_target_var, "W210") == []

    def test_no_false_w210_in_three_handler_group(self):
        source = """\
proc p {} {
    try {error boom} on ok a - on error b - trap NONE c {
        return $b
    }
}
"""
        assert _codes(source, "W210") == []

    def test_genuine_read_before_set_still_fires_in_shared_body(self):
        # A read of a variable that is *not* bound by any handler in the
        # group is still a genuine read-before-set.
        source = """\
proc p {} {
    try {set v 1} on ok x - on error y {
        return $undefinedvar
    }
}
"""
        assert len(_codes(source, "W210")) == 1

    def test_empty_handler_body_is_not_fallthrough(self):
        # A genuinely empty ``{}`` body is valid and distinct from ``-``.
        source = """\
proc p {} {
    try {
        set x 1
    } on ok a {} on error b {
        return $b
    }
}
"""
        assert _e002(source) == []
        ir = lower_to_ir(source)
        stmt = _find_try(ir.procedures["::p"].body)
        assert stmt is not None
        assert stmt.handlers[0].fallthrough is False

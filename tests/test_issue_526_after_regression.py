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

"""Regression test for GitHub issue #526 -- ``after`` command argument (W105).

Issue #526 (tcl-lsp v1.10.10, VSCode) reported a **false positive**: the
recommended, safe list-built form ::

    after 10 [list myCommand $arg1 $arg2]

was flagged W105 ("Code block argument to 'after' is not braced and contains
substitutions -- risk of double substitution. Use braces: { ... }").

A command-substitution body (``[list ...]``) is produced dynamically and parsed
exactly once by ``after`` -- there is no double-substitution risk and it cannot
be braced (``after 10 {[list ...]}`` changes the meaning), so W105 must not
fire.  The fix lives in :func:`analyser.checks._style.check_unbraced_body`.

W105 reaches ``after`` because ``after``'s trailing script argument carries the
``ArgRole.BODY`` role; the audit flagged that this BODY-role wiring is
exercised only generically (``eval [list ...]``) and never through ``after``
specifically.  This file asserts **both** sides so the wiring can't silently
regress:

* **fix holds** -- ``after 10 [list ...]`` (and the ``after idle`` variant)
  raises no W105; and
* **true positive still fires** -- a genuinely-unsafe string-concatenated
  script (``after 10 "myCommand $arg"``) is still flagged W105 as an Error.

The exact diagnostic code involved is **W105** (severity Error for the
dangerous string-concatenated form).  Verified empirically with
``get_diagnostics`` -- do not assume the code letter.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol.types import DiagnosticSeverity

from server.features.diagnostics import get_diagnostics


def _w105(source: str):
    """Return all W105 diagnostics for *source*."""
    return [d for d in get_diagnostics(source) if d.code == "W105"]


class TestIssue526AfterArgument:
    """#526 -- ``after`` script argument and W105 double-substitution warning."""

    def test_fix_holds_list_built_script_not_flagged(self):
        # The exact snippet from issue #526.  Safe list-built command:
        # parsed once by ``after``, no double substitution, cannot be braced.
        assert _w105("after 10 [list myCommand $arg1 $arg2]") == []

    def test_fix_holds_after_idle_list_built_script_not_flagged(self):
        # Same safe form via the ``after idle`` subcommand -- also a BODY arg.
        assert _w105("after idle [list myCommand $arg1 $arg2]") == []

    def test_fix_holds_braced_script_not_flagged(self):
        # An explicitly braced body is the canonical safe form.
        assert _w105("after 10 {myCommand $arg1 $arg2}") == []

    def test_true_positive_quoted_script_still_flagged(self):
        # Genuinely unsafe: a double-quoted script undergoes substitution at
        # call time and again when ``after`` evaluates it -- double
        # substitution.  This must still be flagged W105 (Error).  If the
        # BODY-role wiring for ``after`` regresses, this assertion fails.
        diags = _w105('after 10 "myCommand $arg1 $arg2"')
        assert len(diags) == 1
        assert diags[0].severity is DiagnosticSeverity.Error

    def test_true_positive_after_idle_quoted_script_still_flagged(self):
        diags = _w105('after idle "myCommand $arg1 $arg2"')
        assert len(diags) == 1
        assert diags[0].severity is DiagnosticSeverity.Error

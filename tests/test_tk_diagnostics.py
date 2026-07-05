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

"""Tests for Tk-specific diagnostics (TK1001, TK1002, TK1003)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from analyser.checks.tk import check_tk_diagnostics


class TestTK1001GeometryConflict:
    def test_mixed_pack_grid_same_parent(self):
        source = (
            "package require Tk\n"
            "frame .f\n"
            "button .f.b1 -text Hello\n"
            "button .f.b2 -text World\n"
            "pack .f.b1 -side left\n"
            "grid .f.b2 -row 0 -column 0\n"
        )
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1001" in codes

    def test_pack_only_no_conflict(self):
        source = (
            "package require Tk\n"
            "frame .f\n"
            "button .f.b1 -text Hello\n"
            "button .f.b2 -text World\n"
            "pack .f.b1 -side left\n"
            "pack .f.b2 -side left\n"
        )
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1001" not in codes

    def test_grid_only_no_conflict(self):
        source = (
            "package require Tk\n"
            "frame .f\n"
            "button .f.b1 -text Hello\n"
            "button .f.b2 -text World\n"
            "grid .f.b1 -row 0 -column 0\n"
            "grid .f.b2 -row 0 -column 1\n"
        )
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1001" not in codes

    def test_place_only_no_conflict(self):
        source = (
            "package require Tk\n"
            "frame .f\n"
            "button .f.b1 -text Hello\n"
            "button .f.b2 -text World\n"
            "place .f.b1 -x 0 -y 0\n"
            "place .f.b2 -x 100 -y 0\n"
        )
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1001" not in codes

    def test_different_parents_no_conflict(self):
        """Using pack in one parent and grid in another is fine."""
        source = (
            "package require Tk\n"
            "frame .f1\n"
            "frame .f2\n"
            "button .f1.b1 -text Hello\n"
            "button .f2.b2 -text World\n"
            "pack .f1.b1\n"
            "grid .f2.b2 -row 0 -column 0\n"
        )
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1001" not in codes

    def test_conflict_reports_multiple_diagnostics(self):
        """Each geometry invocation in a conflicting parent gets a diagnostic."""
        source = (
            "package require Tk\n"
            "frame .f\n"
            "button .f.b1 -text Hello\n"
            "button .f.b2 -text World\n"
            "pack .f.b1\n"
            "grid .f.b2 -row 0 -column 0\n"
        )
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        tk1001_diags = [d for d in diags if d.code == "TK1001"]
        # Both the pack and grid invocations produce a TK1001.
        assert len(tk1001_diags) == 2


class TestTK1002InvalidParent:
    def test_missing_parent_widget(self):
        source = "package require Tk\nbutton .nonexistent.btn -text Hello\n"
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1002" in codes

    def test_valid_parent(self):
        source = "package require Tk\nframe .f\nbutton .f.btn -text Hello\n"
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1002" not in codes

    def test_root_widget_always_valid(self):
        source = "package require Tk\nbutton .btn -text Hello\n"
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1002" not in codes

    def test_deeply_nested_missing_parent(self):
        """Parent .a exists but grandparent .a.b does not, so .a.b.c is invalid."""
        source = "package require Tk\nframe .a\nbutton .a.b.c -text Hello\n"
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1002" in codes

    def test_parent_created_before_child(self):
        """When parent is created before child, no diagnostic."""
        source = "package require Tk\nframe .top\nframe .top.mid\nbutton .top.mid.btn -text OK\n"
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1002" not in codes


class TestTK1003UnknownOption:
    def test_unknown_option_detected(self):
        source = "package require Tk\nbutton .btn -text Hello -bogus foo\n"
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1003" in codes

    def test_valid_options_no_diagnostic(self):
        source = "package require Tk\nbutton .btn -text Hello -command exit\n"
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        codes = [d.code for d in diags]
        assert "TK1003" not in codes


class TestNoDiagnosticsWithoutTk:
    def test_no_diagnostics_on_plain_tcl(self):
        """Non-Tk source should produce no Tk diagnostics at all."""
        source = "set x 42\nputs $x\n"
        result = analyse(source)
        diags = check_tk_diagnostics(source, result)
        assert diags == []


class TestTkGeometryManagerShortcut:
    """``grid pathName ?args?`` / ``pack pathName ?args?`` / ``place pathName
    ?args?`` are documented Tk shortcuts for the ``configure`` subcommand
    (per Tk man pages grid.n / pack.n / place.n).  A window path starts with
    ``.`` (Tk's path convention), which is not a valid Tcl subcommand-name
    first character — so the analyser must NOT raise W001
    ('Unknown subcommand') on the shortcut form."""

    @staticmethod
    def _codes(src: str) -> list[tuple[str, str]]:
        from server.features.diagnostics import get_diagnostics

        return [
            (str(d.code), d.message)
            for d in get_diagnostics(src)
            if isinstance(d.code, str) and d.code == "W001"
        ]

    def test_grid_window_path_no_w001(self):
        assert self._codes("grid .x\n") == []
        assert self._codes("grid .x -row 0 -column 0\n") == []
        assert self._codes("grid .frame.button -sticky ew\n") == []

    def test_pack_window_path_no_w001(self):
        assert self._codes("pack .x\n") == []
        assert self._codes("pack .x -side left\n") == []

    def test_place_window_path_no_w001(self):
        assert self._codes("place .x -x 0 -y 0\n") == []

    def test_genuine_typo_still_flagged(self):
        # TP control: a real subcommand typo must still fire.  ``colmconfigure``
        # is a typo for ``columnconfigure`` — no window path, no exemption.
        codes = self._codes("grid colmconfigure . 0 -weight 1\n")
        assert any("Unknown subcommand 'colmconfigure'" in m for _, m in codes)

    def test_real_subcommand_still_silent(self):
        # FP control: a real subcommand (``info ?pathName?``) is silent.
        assert self._codes("grid info .x\n") == []
        assert self._codes("grid columnconfigure . 0 -weight 1\n") == []


class TestTkGateWhitespaceVariants:
    """The cheap pre-``analyse()`` gate in ``get_diagnostics`` must recognise a
    ``package require Tk`` written with any word separator, not just a single
    space — the lexer splits words on ``\\t``/multiple spaces/``;`` and a
    backslash-newline continuation, so those are all real requires (tclsh
    tokenises identically).  Missing them silently dropped TK100x (the pre-A8
    unconditional analyse() caught them)."""

    _BASE = (
        "package require Tk\n"
        "frame .f\n"
        "button .f.b1 -text Hello\n"
        "button .f.b2 -text World\n"
        "pack .f.b1 -side left\n"
        "grid .f.b2 -row 0 -column 0\n"
    )

    @staticmethod
    def _tk_codes(src: str) -> set[str]:
        from server.features.diagnostics import get_diagnostics

        return {
            d.code
            for d in get_diagnostics(src)
            if isinstance(d.code, str) and d.code.startswith("TK")
        }

    def test_single_space_fires_tk1001(self):
        assert "TK1001" in self._tk_codes(self._BASE)

    def test_tab_separated_require_still_fires(self):
        src = self._BASE.replace("package require Tk", "package\trequire Tk")
        assert "TK1001" in self._tk_codes(src)

    def test_double_space_require_still_fires(self):
        src = self._BASE.replace("package require Tk", "package  require Tk")
        assert "TK1001" in self._tk_codes(src)

    def test_line_continued_require_still_fires(self):
        src = self._BASE.replace("package require Tk", "package \\\n    require Tk")
        assert "TK1001" in self._tk_codes(src)

    def test_non_tk_file_still_silent(self):
        assert self._tk_codes("set x 1\nputs hello\n") == set()

"""Tests for Tk-specific diagnostics (TK1001, TK1002, TK1003)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.tk.diagnostics import check_tk_diagnostics


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

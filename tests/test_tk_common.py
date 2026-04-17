"""Tests for shared Tk constants/path helpers."""

from __future__ import annotations

from core.analysis.analyser import analyse
from core.tk.common import is_widget_path, parent_widget_path
from core.tk.diagnostics import check_tk_diagnostics
from core.tk.extract import extract_tk_layout


def test_widget_path_validation_and_parent_derivation() -> None:
    assert not is_widget_path(".")
    assert is_widget_path(".frame.child")
    assert not is_widget_path("frame.child")
    assert not is_widget_path(".1bad")

    assert parent_widget_path(".") == ""
    assert parent_widget_path(".child") == "."
    assert parent_widget_path(".root.child") == ".root"


def test_tk_extract_and_diagnostics_share_widget_path_classification() -> None:
    valid_source = "package require Tk\nframe .f\nbutton .f.b -text Ok\n"
    layout = extract_tk_layout(valid_source)
    diagnostics = check_tk_diagnostics(valid_source, analyse(valid_source))
    codes = {d.code for d in diagnostics}

    assert layout["widget_count"] >= 3  # root + frame + button
    assert "TK1002" not in codes

    invalid_source = "package require Tk\nbutton .1bad -text Nope\n"
    invalid_layout = extract_tk_layout(invalid_source)
    invalid_diags = check_tk_diagnostics(invalid_source, analyse(invalid_source))
    invalid_codes = {d.code for d in invalid_diags}

    # Both modules should treat invalid widget paths as non-widget targets.
    assert invalid_layout["widget_count"] == 1
    assert "TK1002" not in invalid_codes

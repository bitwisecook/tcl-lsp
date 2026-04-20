"""Tests for Tk widget tree extraction."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.tk.extract import extract_tk_layout


class TestExtractTkLayout:
    def test_empty_source(self):
        result = extract_tk_layout("")
        # Root widget "." always exists, so count is 1 even for empty source.
        assert result["widget_count"] == 1
        assert result["root"]["pathname"] == "."
        assert result["root"]["type"] == "toplevel"
        assert result["geometry_conflicts"] == []

    def test_simple_button(self):
        source = 'package require Tk\nwm title . "Test"\nbutton .btn -text "Click me"\npack .btn\n'
        result = extract_tk_layout(source)
        assert result["widget_count"] >= 2  # root + button
        assert result["root"]["pathname"] == "."
        assert result["root"]["title"] == "Test"
        # The button should be a child of root.
        children = result["root"]["children"]
        assert len(children) >= 1
        btn = children[0]
        assert btn["pathname"] == ".btn"
        assert btn["type"] == "button"

    def test_button_geometry_tracked(self):
        source = 'package require Tk\nbutton .btn -text "Hello"\npack .btn -side left\n'
        result = extract_tk_layout(source)
        btn = result["root"]["children"][0]
        assert btn["geometry"] == "pack"
        assert btn["geometry_options"].get("-side") == "left"

    def test_grid_layout(self):
        source = (
            "package require Tk\n"
            "frame .f\n"
            'label .f.l -text "Name:"\n'
            "entry .f.e\n"
            "grid .f.l -row 0 -column 0\n"
            "grid .f.e -row 0 -column 1\n"
            "grid .f -row 0 -column 0\n"
        )
        result = extract_tk_layout(source)
        # Root + frame + label + entry = 4 widgets.
        assert result["widget_count"] >= 4

    def test_grid_options_captured(self):
        source = "package require Tk\nframe .f\ngrid .f -row 0 -column 0\n"
        result = extract_tk_layout(source)
        frame = result["root"]["children"][0]
        assert frame["geometry"] == "grid"
        assert frame["geometry_options"]["-row"] == "0"
        assert frame["geometry_options"]["-column"] == "0"

    def test_geometry_conflicts_detected(self):
        source = (
            "package require Tk\n"
            "frame .f\n"
            'button .f.b1 -text "A"\n'
            'button .f.b2 -text "B"\n'
            "pack .f.b1\n"
            "grid .f.b2 -row 0 -column 0\n"
        )
        result = extract_tk_layout(source)
        assert len(result["geometry_conflicts"]) > 0
        assert "pack" in result["geometry_conflicts"][0]
        assert "grid" in result["geometry_conflicts"][0]

    def test_no_geometry_conflicts_same_manager(self):
        source = (
            "package require Tk\n"
            "frame .f\n"
            'button .f.b1 -text "A"\n'
            'button .f.b2 -text "B"\n'
            "pack .f.b1\n"
            "pack .f.b2\n"
        )
        result = extract_tk_layout(source)
        assert result["geometry_conflicts"] == []

    def test_visual_options_captured(self):
        source = 'package require Tk\nbutton .btn -text "Hello" -width 20\n'
        result = extract_tk_layout(source)
        btn = result["root"]["children"][0]
        assert btn["options"].get("-text") == '"Hello"' or btn["options"].get("-text") == "Hello"
        assert btn["options"].get("-width") == "20"

    def test_ttk_widget_type(self):
        source = 'package require Tk\nttk::button .btn -text "Hello"\n'
        result = extract_tk_layout(source)
        btn = result["root"]["children"][0]
        assert btn["type"] == "ttk-button"

    def test_wm_title_extraction(self):
        source = 'package require Tk\nwm title . "My Application"\n'
        result = extract_tk_layout(source)
        assert result["root"]["title"] == "My Application"

    def test_default_title(self):
        source = "package require Tk\n"
        result = extract_tk_layout(source)
        assert result["root"]["title"] == "Tk"

    def test_nested_children(self):
        source = 'package require Tk\nframe .f\nbutton .f.btn -text "OK"\n'
        result = extract_tk_layout(source)
        frame = result["root"]["children"][0]
        assert frame["pathname"] == ".f"
        assert len(frame["children"]) == 1
        btn = frame["children"][0]
        assert btn["pathname"] == ".f.btn"

    def test_comments_ignored(self):
        source = 'package require Tk\n# This is a comment\nbutton .btn -text "Hello"\n'
        result = extract_tk_layout(source)
        assert result["widget_count"] == 2  # root + button


class TestExtractTkLayoutEdgeCases:
    def test_source_with_only_comments(self):
        result = extract_tk_layout("# just a comment\n# another one\n")
        assert result["widget_count"] == 1  # Only root.
        assert result["geometry_conflicts"] == []

    def test_place_geometry(self):
        source = 'package require Tk\nbutton .btn -text "Hello"\nplace .btn -x 10 -y 20\n'
        result = extract_tk_layout(source)
        btn = result["root"]["children"][0]
        assert btn["geometry"] == "place"
        assert btn["geometry_options"].get("-x") == "10"
        assert btn["geometry_options"].get("-y") == "20"

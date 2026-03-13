"""Tests for the Tk command registry."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.commands.registry.command_registry import CommandRegistry
from core.commands.registry.tk import tk_command_specs


class TestTkRegistry:
    def test_tk_specs_not_empty(self):
        specs = tk_command_specs()
        assert len(specs) > 0

    def test_all_specs_require_tk(self):
        for spec in tk_command_specs():
            assert spec.required_package == "Tk", f"{spec.name} missing required_package"

    def test_key_commands_present(self):
        names = {s.name for s in tk_command_specs()}
        expected = {
            "button",
            "label",
            "entry",
            "text",
            "frame",
            "canvas",
            "pack",
            "grid",
            "place",
            "wm",
            "winfo",
            "ttk::button",
            "ttk::treeview",
            "ttk::style",
            "tk_messageBox",
        }
        for cmd in expected:
            assert cmd in names, f"Missing Tk command: {cmd}"

    def test_hover_docs_present(self):
        for spec in tk_command_specs():
            assert spec.hover is not None, f"{spec.name} missing hover"
            assert spec.hover.summary, f"{spec.name} has empty hover summary"

    def test_widget_commands_have_options(self):
        widgets = {"button", "label", "entry", "text", "frame", "canvas"}
        specs = {s.name: s for s in tk_command_specs()}
        for name in widgets:
            spec = specs[name]
            assert spec.forms, f"{name} has no forms"
            assert spec.forms[0].options, f"{name} has no options"

    def test_ttk_commands_have_options(self):
        ttk_widgets = {"ttk::button", "ttk::treeview", "ttk::entry"}
        specs = {s.name: s for s in tk_command_specs()}
        for name in ttk_widgets:
            spec = specs[name]
            assert spec.forms, f"{name} has no forms"
            assert spec.forms[0].options, f"{name} has no options"

    def test_geometry_managers_present(self):
        names = {s.name for s in tk_command_specs()}
        for cmd in ("pack", "grid", "place"):
            assert cmd in names, f"Missing geometry manager: {cmd}"

    def test_no_duplicate_command_names(self):
        names = [s.name for s in tk_command_specs()]
        assert len(names) == len(set(names)), "Duplicate command names in Tk specs"


class TestCommandsForPackages:
    def setup_method(self):
        self.registry = CommandRegistry.build_default()

    def test_no_tk_without_package(self):
        names = set(self.registry.commands_for_packages(frozenset()))
        assert "button" not in names
        assert "ttk::button" not in names
        # Core Tcl commands should still be present.
        assert "set" in names

    def test_tk_with_package(self):
        names = set(self.registry.commands_for_packages(frozenset({"Tk"})))
        assert "button" in names
        assert "ttk::button" in names
        assert "set" in names  # Core Tcl still present.

    def test_tk_commands_appear_with_tk_package_only(self):
        """Tk commands must not appear when only non-Tk packages are required."""
        names = set(self.registry.commands_for_packages(frozenset({"http"})))
        assert "button" not in names
        assert "pack" not in names

    def test_multiple_packages_include_tk(self):
        """When Tk is among multiple packages, Tk commands should be present."""
        names = set(self.registry.commands_for_packages(frozenset({"Tk", "http"})))
        assert "button" in names
        assert "wm" in names

    def test_registry_get_finds_tk_commands(self):
        """Tk commands should be findable via registry.get() for hover/validation."""
        spec = self.registry.get("button")
        assert spec is not None
        assert spec.required_package == "Tk"

    def test_registry_switches_for_tk_widget(self):
        """Tk widget commands should have switch metadata."""
        switches = self.registry.switches("button")
        assert "-text" in switches
        assert "-command" in switches

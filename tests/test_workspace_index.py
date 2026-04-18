"""Tests for the workspace index."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from lsp.workspace.workspace_index import EntrySource, WorkspaceIndex


class TestWorkspaceIndex:
    def test_update_and_find(self):
        idx = WorkspaceIndex()
        result = analyse("proc greet {name} { puts $name }")
        idx.update("file:///a.tcl", result)
        entries = idx.find_proc("greet")
        assert len(entries) == 1
        assert entries[0].uri == "file:///a.tcl"
        assert entries[0].proc is not None
        assert entries[0].proc.name == "greet"

    def test_find_by_qualified_name(self):
        idx = WorkspaceIndex()
        result = analyse("proc greet {name} { puts $name }")
        idx.update("file:///a.tcl", result)
        entries = idx.find_proc("::greet")
        assert len(entries) == 1

    def test_find_by_simple_name_tail(self):
        idx = WorkspaceIndex()
        source = """namespace eval math { proc add {a b} { return [+ $a $b] } }"""
        result = analyse(source)
        idx.update("file:///a.tcl", result)
        entries = idx.find_proc("add")
        assert len(entries) == 1

    def test_remove(self):
        idx = WorkspaceIndex()
        result = analyse("proc greet {name} { puts $name }")
        idx.update("file:///a.tcl", result)
        assert len(idx.find_proc("greet")) == 1
        idx.remove("file:///a.tcl")
        assert len(idx.find_proc("greet")) == 0

    def test_all_proc_names(self):
        idx = WorkspaceIndex()
        result = analyse("proc foo {} {}\nproc bar {} {}")
        idx.update("file:///a.tcl", result)
        names = idx.all_proc_names()
        assert "::foo" in names
        assert "::bar" in names

    def test_multi_file(self):
        idx = WorkspaceIndex()
        idx.update("file:///a.tcl", analyse("proc foo {} {}"))
        idx.update("file:///b.tcl", analyse("proc bar {} {}"))
        assert len(idx.find_proc("foo")) == 1
        assert len(idx.find_proc("bar")) == 1
        assert len(idx.all_proc_names()) == 2

    def test_update_replaces(self):
        idx = WorkspaceIndex()
        idx.update("file:///a.tcl", analyse("proc foo {} {}"))
        idx.update("file:///a.tcl", analyse("proc bar {} {}"))
        assert len(idx.find_proc("foo")) == 0
        assert len(idx.find_proc("bar")) == 1

    def test_get_analysis(self):
        idx = WorkspaceIndex()
        result = analyse("set x 42")
        idx.update("file:///a.tcl", result)
        assert idx.get_analysis("file:///a.tcl") is result
        assert idx.get_analysis("file:///b.tcl") is None

    def test_command_usage_counts(self):
        idx = WorkspaceIndex()
        source = "proc helper {} { puts ok }\nhelper\nhelper\nputs hi\n"
        idx.update("file:///a.tcl", analyse(source))
        counts = idx.command_usage_counts()
        assert counts["helper"] == 2
        assert counts["puts"] == 2
        assert counts["proc"] == 1

    def test_proc_usage_counts(self):
        idx = WorkspaceIndex()
        source = "proc helper {} { return ok }\nhelper\nhelper\n"
        idx.update("file:///a.tcl", analyse(source))
        counts = idx.proc_usage_counts()
        assert counts["::helper"] == 2

    def test_usage_counts_update_and_remove(self):
        idx = WorkspaceIndex()
        idx.update("file:///a.tcl", analyse("proc helper {} {}\nhelper\nhelper\n"))
        assert idx.command_usage_counts().get("helper") == 2
        assert idx.proc_usage_counts().get("::helper") == 2

        idx.update("file:///a.tcl", analyse("proc helper {} {}\nhelper\n"))
        assert idx.command_usage_counts().get("helper") == 1
        assert idx.proc_usage_counts().get("::helper") == 1

        idx.remove("file:///a.tcl")
        assert idx.command_usage_counts().get("helper") is None
        assert idx.proc_usage_counts().get("::helper") is None


class TestEntrySource:
    def test_default_source_is_open(self):
        idx = WorkspaceIndex()
        idx.update("file:///a.tcl", analyse("proc foo {} {}"))
        assert not idx.is_background("file:///a.tcl")

    def test_background_source(self):
        idx = WorkspaceIndex()
        idx.update(
            "file:///a.tcl",
            analyse("proc foo {} {}"),
            EntrySource.BACKGROUND,
        )
        assert idx.is_background("file:///a.tcl")

    def test_package_source(self):
        idx = WorkspaceIndex()
        idx.update(
            "file:///a.tcl",
            analyse("proc foo {} {}"),
            EntrySource.PACKAGE,
        )
        assert idx.is_background("file:///a.tcl")

    def test_open_overrides_background(self):
        idx = WorkspaceIndex()
        idx.update(
            "file:///a.tcl",
            analyse("proc foo {} {}"),
            EntrySource.BACKGROUND,
        )
        assert idx.is_background("file:///a.tcl")
        # Opening the file replaces the background entry
        idx.update(
            "file:///a.tcl",
            analyse("proc foo {} { puts hello }"),
            EntrySource.OPEN,
        )
        assert not idx.is_background("file:///a.tcl")
        # Proc is still findable
        assert len(idx.find_proc("foo")) == 1

    def test_remove_background_entries(self):
        idx = WorkspaceIndex()
        idx.update("file:///a.tcl", analyse("proc foo {} {}"), EntrySource.OPEN)
        idx.update("file:///b.tcl", analyse("proc bar {} {}"), EntrySource.BACKGROUND)
        idx.update("file:///c.tcl", analyse("proc baz {} {}"), EntrySource.PACKAGE)

        idx.remove_background_entries()

        # OPEN entry survives
        assert len(idx.find_proc("foo")) == 1
        # BACKGROUND and PACKAGE entries removed
        assert len(idx.find_proc("bar")) == 0
        assert len(idx.find_proc("baz")) == 0

    def test_all_uris(self):
        idx = WorkspaceIndex()
        idx.update("file:///a.tcl", analyse("proc foo {} {}"), EntrySource.OPEN)
        idx.update("file:///b.tcl", analyse("proc bar {} {}"), EntrySource.BACKGROUND)
        uris = idx.all_uris()
        assert "file:///a.tcl" in uris
        assert "file:///b.tcl" in uris


class TestIrulesGlobals:
    def test_irules_globals_findable(self):
        idx = WorkspaceIndex()
        result = analyse("proc helper {} {}")
        idx.update_irules_globals("file:///rule.irul", result.all_procs)
        entries = idx.find_proc("helper")
        assert len(entries) == 1
        assert entries[0].uri == "file:///rule.irul"

    def test_irules_globals_in_all_proc_names(self):
        idx = WorkspaceIndex()
        result = analyse("proc helper {} {}")
        idx.update_irules_globals("file:///rule.irul", result.all_procs)
        names = idx.all_proc_names()
        assert "::helper" in names

    def test_irules_globals_deduplicated(self):
        """If same proc is in both regular index and irules globals, deduplicate."""
        idx = WorkspaceIndex()
        result = analyse("proc handler {} {}")
        idx.update("file:///rule.irul", result, EntrySource.BACKGROUND)
        idx.update_irules_globals("file:///rule.irul", result.all_procs)
        entries = idx.find_proc("handler")
        # Should be deduplicated
        assert len(entries) == 1

    def test_irules_globals_update_replaces(self):
        idx = WorkspaceIndex()
        result1 = analyse("proc old_helper {} {}")
        idx.update_irules_globals("file:///rule.irul", result1.all_procs)
        assert len(idx.find_proc("old_helper")) == 1

        result2 = analyse("proc new_helper {} {}")
        idx.update_irules_globals("file:///rule.irul", result2.all_procs)
        assert len(idx.find_proc("old_helper")) == 0
        assert len(idx.find_proc("new_helper")) == 1

    def test_irules_globals_cleared_on_remove_background(self):
        idx = WorkspaceIndex()
        result = analyse("proc helper {} {}")
        idx.update_irules_globals("file:///rule.irul", result.all_procs)
        assert len(idx.find_proc("helper")) == 1
        idx.remove_background_entries()
        assert len(idx.find_proc("helper")) == 0

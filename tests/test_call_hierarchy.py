"""Tests for the call hierarchy provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from server.features.call_hierarchy import (
    incoming_calls,
    outgoing_calls,
    prepare_call_hierarchy,
)

TEST_URI = "file:///test.tcl"


class TestPrepareCallHierarchy:
    def test_on_proc_definition(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        items = prepare_call_hierarchy(source, TEST_URI, 0, 6)
        assert len(items) == 1
        assert items[0].name == "greet"
        assert items[0].kind == types.SymbolKind.Function

    def test_on_proc_call(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        items = prepare_call_hierarchy(source, TEST_URI, 1, 0)
        assert len(items) == 1
        assert items[0].name == "greet"

    def test_on_builtin(self):
        source = "puts hello"
        items = prepare_call_hierarchy(source, TEST_URI, 0, 1)
        assert len(items) == 0

    def test_empty_file(self):
        items = prepare_call_hierarchy("", TEST_URI, 0, 0)
        assert len(items) == 0


class TestIncomingCalls:
    def test_find_callers(self):
        source = textwrap.dedent("""\
            proc greet {} { return }
            proc main {} { greet }
            greet
        """)
        items = prepare_call_hierarchy(source, TEST_URI, 0, 6)
        assert len(items) == 1
        calls = incoming_calls(items[0], source, TEST_URI)
        assert len(calls) >= 1
        caller_names = {c.from_.name for c in calls}
        assert "main" in caller_names or "<top-level>" in caller_names

    def test_cross_document_callers_included(self):
        from analyser import analyse

        # The proc is defined in one file and called from another.
        def_uri = "file:///lib.tcl"
        def_source = "proc greet {} { return }\n"
        caller_uri = "file:///app.tcl"
        caller_source = "proc main {} { greet }\n"

        items = prepare_call_hierarchy(def_source, def_uri, 0, 6)
        assert len(items) == 1

        # Without the other document, the cross-file caller is missing.
        local_only = incoming_calls(items[0], def_source, def_uri)
        assert all(c.from_.name != "main" for c in local_only)

        # Supplying the other document surfaces its caller, attributed to
        # that file's URI.
        with_extra = incoming_calls(
            items[0],
            def_source,
            def_uri,
            extra_documents=[(caller_uri, analyse(caller_source))],
        )
        main_calls = [c for c in with_extra if c.from_.name == "main"]
        assert len(main_calls) == 1
        assert main_calls[0].from_.uri == caller_uri


class TestOutgoingCalls:
    def test_find_callees(self):
        source = textwrap.dedent("""\
            proc helper {} { return 1 }
            proc main {} {
                helper
            }
        """)
        items = prepare_call_hierarchy(source, TEST_URI, 1, 6)
        assert len(items) == 1
        calls = outgoing_calls(items[0], source, TEST_URI)
        assert len(calls) >= 1
        callee_names = {c.to.name for c in calls}
        assert "helper" in callee_names

    def test_no_outgoing_for_leaf_proc(self):
        source = textwrap.dedent("""\
            proc leaf {} { return 1 }
        """)
        items = prepare_call_hierarchy(source, TEST_URI, 0, 6)
        assert len(items) == 1
        calls = outgoing_calls(items[0], source, TEST_URI)
        assert len(calls) == 0

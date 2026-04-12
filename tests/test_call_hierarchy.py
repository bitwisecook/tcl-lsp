"""Tests for the call hierarchy provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.call_hierarchy import (
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

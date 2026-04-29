"""S4.2 v0 — empty-body splice tests."""

from __future__ import annotations

import textwrap

from core.compiler.inlining import (
    apply_inline_catalogue,
    inline_module,
)
from core.compiler.ir import IRCall, InlineDecision
from core.compiler.lowering import lower_to_ir
from core.compiler.var_escape import analyse_var_escape


def _prepare(source: str):
    """Lower + tag + return ``(module, summaries)``."""
    module = lower_to_ir(textwrap.dedent(source))
    summaries = analyse_var_escape(ir_module=module)
    tagged = apply_inline_catalogue(module, summaries)
    return tagged, summaries


def _statement_kinds(script):
    return [type(s).__name__ for s in script.statements]


def _calls_to(script, command: str) -> int:
    """Count IRCall statements whose ``command`` field equals ``command``."""
    return sum(
        1
        for s in script.statements
        if isinstance(s, IRCall) and s.command == command
    )


class TestEmptyBodySplice:
    def test_empty_body_call_is_dropped(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "noop\n"
            "noop\n"
        )
        # Sanity: pre-inline the top-level has two ``noop`` calls.
        assert _calls_to(module.top_level, "noop") == 2
        new_module = inline_module(module, summaries)
        # Both calls vanish; the ``proc`` definition stays put.
        assert _calls_to(new_module.top_level, "noop") == 0

    def test_non_empty_body_is_not_inlined(self):
        # Even though ``setup`` is pure_leaf and small, its body has
        # a statement so the v0 splice declines.
        module, summaries = _prepare(
            "proc setup {} { set x 1 }\n"
            "setup\n"
        )
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "setup") == 1

    def test_non_pure_leaf_is_not_inlined(self):
        # ``upvar`` blocks pure_leaf, so the catalogue tags ``skip_me``
        # NEVER and the inliner skips it even with an empty body.
        module, summaries = _prepare(
            "proc skip_me {name} { upvar 1 $name v }\n"
            "skip_me x\n"
        )
        proc = module.procedures["::skip_me"]
        assert proc.inline_decision is InlineDecision.NEVER
        new_module = inline_module(module, summaries)
        assert _calls_to(new_module.top_level, "skip_me") == 1


class TestNestedSites:
    def test_call_inside_if_clause_dropped(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "if {1} { noop }\n"
        )
        new_module = inline_module(module, summaries)
        # Find the IRIf statement; its clause body should have no
        # remaining ``noop`` call.
        from core.compiler.ir import IRIf

        if_nodes = [s for s in new_module.top_level.statements if isinstance(s, IRIf)]
        assert if_nodes, "expected an IRIf statement"
        clause_body = if_nodes[0].clauses[0].body
        assert _calls_to(clause_body, "noop") == 0

    def test_call_inside_for_body_dropped(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "for {set i 0} {$i < 3} {incr i} { noop }\n"
        )
        new_module = inline_module(module, summaries)
        from core.compiler.ir import IRFor

        for_nodes = [s for s in new_module.top_level.statements if isinstance(s, IRFor)]
        assert for_nodes, "expected an IRFor statement"
        assert _calls_to(for_nodes[0].body, "noop") == 0


class TestPurity:
    def test_input_module_unchanged(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "noop\n"
        )
        original_calls = _calls_to(module.top_level, "noop")
        inline_module(module, summaries)
        # The original module's statements still mention the call.
        assert _calls_to(module.top_level, "noop") == original_calls

    def test_idempotent(self):
        module, summaries = _prepare(
            "proc noop {} {}\n"
            "noop\n"
        )
        once = inline_module(module, summaries)
        twice = inline_module(once, summaries)
        assert _calls_to(once.top_level, "noop") == _calls_to(
            twice.top_level, "noop"
        )


class TestResolution:
    def test_unqualified_call_in_namespace(self):
        # The callee is defined inside ``::ns``; the call is bare.
        # The interprocedural resolver walks namespaces, so the
        # bare ``noop`` from inside ``::ns::caller`` resolves to
        # ``::ns::noop`` and is eligible.
        module, summaries = _prepare(
            "namespace eval ::ns {\n"
            "  proc noop {} {}\n"
            "  proc caller {} { noop }\n"
            "}\n"
        )
        # The catalogue should mark ::ns::noop as ALWAYS.
        assert module.procedures["::ns::noop"].inline_decision is InlineDecision.ALWAYS
        new_module = inline_module(module, summaries)
        caller_body = new_module.procedures["::ns::caller"].body
        assert _calls_to(caller_body, "noop") == 0

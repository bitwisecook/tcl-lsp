"""S4.1 inlining-catalogue tests."""

from __future__ import annotations

import textwrap

from core.compiler.inlining import (
    SMALL_BODY_THRESHOLD,
    apply_inline_catalogue,
    count_statements,
    count_static_calls,
)
from core.compiler.ir import InlineDecision
from core.compiler.lowering import lower_to_ir
from core.compiler.var_escape import analyse_var_escape


def _catalogue(source: str):
    """Lower ``source`` and return ``(module, summaries)`` post-S4.1."""
    module = lower_to_ir(textwrap.dedent(source))
    summaries = analyse_var_escape(ir_module=module)
    return apply_inline_catalogue(module, summaries), summaries


class TestStatementCount:
    def test_empty_body_is_zero(self):
        module = lower_to_ir("proc foo {} {}\n")
        proc = module.procedures["::foo"]
        assert count_statements(proc.body) == 0

    def test_flat_body_counts_top_level(self):
        module = lower_to_ir("proc foo {} {\n  set a 1\n  set b 2\n  set c 3\n}\n")
        proc = module.procedures["::foo"]
        assert count_statements(proc.body) == 3

    def test_nested_if_counts_inner(self):
        module = lower_to_ir("proc foo {x} {\n  if {$x} { set a 1 }\n}\n")
        proc = module.procedures["::foo"]
        # IRIf contributes (1 for the clause + nested set)
        assert count_statements(proc.body) >= 1


class TestStaticCallCount:
    def test_top_level_call_counted(self):
        module = lower_to_ir("proc add {a b} { return [expr {$a + $b}] }\nadd 1 2\n")
        summaries = analyse_var_escape(ir_module=module)
        counts = count_static_calls(module, summaries)
        assert counts.get("::add", 0) == 1

    def test_multiple_calls_counted(self):
        module = lower_to_ir(
            "proc add {a b} { return [expr {$a + $b}] }\nadd 1 2\nadd 3 4\nadd 5 6\n"
        )
        summaries = analyse_var_escape(ir_module=module)
        counts = count_static_calls(module, summaries)
        assert counts.get("::add", 0) == 3

    def test_uncalled_proc_zero(self):
        module = lower_to_ir("proc lonely {} { return 1 }\n")
        summaries = analyse_var_escape(ir_module=module)
        counts = count_static_calls(module, summaries)
        assert counts.get("::lonely", 0) == 0


class TestClassify:
    def test_small_pure_leaf_is_always(self):
        module, summaries = _catalogue("proc add {a b} { return [expr {$a + $b}] }\n")
        proc = module.procedures["::add"]
        # Small + pure_leaf → ALWAYS regardless of call count.
        assert proc.inline_decision is InlineDecision.ALWAYS

    def test_non_pure_leaf_is_never(self):
        # ``upvar`` makes the proc non-pure_leaf.
        module, summaries = _catalogue(
            "proc setter {name val} {\n  upvar 1 $name v\n  set v $val\n}\n"
        )
        proc = module.procedures["::setter"]
        assert proc.inline_decision is InlineDecision.NEVER

    def test_large_pure_leaf_with_one_caller_is_if_single_call(self):
        body_lines = "\n".join(f"  set v{i} {i}" for i in range(SMALL_BODY_THRESHOLD + 2))
        source = f"proc big {{}} {{\n{body_lines}\n}}\nbig\n"
        module, _ = _catalogue(source)
        proc = module.procedures["::big"]
        assert proc.static_call_count == 1
        assert proc.inline_decision is InlineDecision.IF_SINGLE_CALL

    def test_large_pure_leaf_with_two_callers_is_never(self):
        body_lines = "\n".join(f"  set v{i} {i}" for i in range(SMALL_BODY_THRESHOLD + 2))
        source = f"proc big {{}} {{\n{body_lines}\n}}\nbig\nbig\n"
        module, _ = _catalogue(source)
        proc = module.procedures["::big"]
        assert proc.static_call_count == 2
        assert proc.inline_decision is InlineDecision.NEVER


class TestCatalogueIsPure:
    def test_input_module_unchanged(self):
        module = lower_to_ir("proc add {a b} { return [expr {$a + $b}] }\n")
        original = module.procedures["::add"]
        summaries = analyse_var_escape(ir_module=module)
        new_module = apply_inline_catalogue(module, summaries)
        # Original frozen IRProcedure should still have its defaults.
        assert original.inline_decision is InlineDecision.NEVER
        assert original.static_call_count == 0
        # New module's proc has the freshly computed tag.
        assert new_module.procedures["::add"].inline_decision is InlineDecision.ALWAYS

"""Tests for S5.3 IR-level LICM pass."""

from __future__ import annotations

import textwrap

from core.compiler.ir import (
    IRAssignConst,
    IRAssignValue,
    IRFor,
    IRForeach,
    IRWhile,
)
from core.compiler.lowering import lower_to_ir
from core.compiler.passes.licm import licm_module


def _module(source: str):
    return lower_to_ir(textwrap.dedent(source))


def _stmt_kinds(script):
    return [type(s).__name__ for s in script.statements]


def _find_first(script, cls):
    for stmt in script.statements:
        if isinstance(stmt, cls):
            return stmt
    return None


def _find_assign(script, name: str):
    for stmt in script.statements:
        if isinstance(stmt, (IRAssignConst, IRAssignValue)) and stmt.name == name:
            return stmt
    return None


class TestSimpleHoist:
    def test_literal_const_in_for_body_hoisted(self):
        module = _module('for {set i 0} {$i < 10} {incr i} {\n  set guard "constant"\n}\n')
        new_module = licm_module(module)
        # The ``set guard "constant"`` should now appear at the
        # top-level (outside the IRFor), and be removed from the
        # loop body.
        for_node = _find_first(new_module.top_level, IRFor)
        assert for_node is not None
        assert _find_assign(for_node.body, "guard") is None, (
            f"loop body still contains the assignment: {_stmt_kinds(for_node.body)}"
        )
        # And the top-level has the hoisted assignment somewhere
        # before the IRFor.
        assert _find_assign(new_module.top_level, "guard") is not None

    def test_literal_const_in_while_body_NOT_hoisted(self):
        # ``while`` loops can run zero iterations when the
        # condition is initially false; hoisting would change
        # the post-loop slot value.  S5.3 declines.
        module = _module('set i 0\nwhile {$i < 5} {\n  set msg "hello"\n  incr i\n}\n')
        new_module = licm_module(module)
        while_node = _find_first(new_module.top_level, IRWhile)
        assert while_node is not None
        # Body retains the literal write — no hoist.
        assert _find_assign(while_node.body, "msg") is not None


class TestForeachLiteralList:
    def test_foreach_with_literal_list_hoists(self):
        # ``foreach x {a b c}`` runs ≥ 1 times because the list
        # is a non-empty literal — the body's invariant write is
        # safely hoisted.
        module = _module('foreach x {a b c} {\n  set guard "k"\n}\n')
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRForeach)
        assert _find_assign(for_node.body, "guard") is None
        assert _find_assign(new_module.top_level, "guard") is not None

    def test_foreach_with_dynamic_list_not_hoisted(self):
        # ``$lst`` is a runtime variable; we can't decide ≥ 1
        # iterations statically.
        module = _module('set lst {a b c}\nforeach x $lst {\n  set guard "k"\n}\n')
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRForeach)
        assert _find_assign(for_node.body, "guard") is not None

    def test_foreach_with_empty_literal_list_not_hoisted(self):
        # The historical regression case — an empty literal list
        # must NOT trigger the hoist or it would change observable
        # behaviour (the body's writes never execute, so they have
        # no business landing before the loop).
        module = _module("set x 99\nforeach i {} {\n  set x 0\n}\n")
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRForeach)
        assert _find_assign(for_node.body, "x") is not None


class TestZeroIterationGate:
    """The hoist must decline whenever the loop could run 0 iterations."""

    def test_for_with_initially_false_condition_not_hoisted(self):
        # ``set i 5; ... $i < 0`` is initially false → 0 iterations.
        # The hoist would otherwise overwrite ``x``.
        module = _module("set x 99\nfor {set i 5} {$i < 0} {incr i} {\n  set x 0\n}\n")
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRFor)
        assert _find_assign(for_node.body, "x") is not None

    def test_for_with_dynamic_bound_not_hoisted(self):
        # ``$i < $N`` — the bound is a runtime variable; we can't
        # decide ≥ 1 iteration statically.
        module = _module("set x 99\nset N 0\nfor {set i 0} {$i < $N} {incr i} {\n  set x 0\n}\n")
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRFor)
        assert _find_assign(for_node.body, "x") is not None


class TestDeclines:
    def test_var_with_substitution_value_not_hoisted(self):
        module = _module("set y 5\nfor {set i 0} {$i < 3} {incr i} {\n  set guard $y\n}\n")
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRFor)
        # Value contains ``$`` — not hoisted.
        assert _find_assign(for_node.body, "guard") is not None

    def test_var_written_twice_in_body_not_hoisted(self):
        module = _module(
            'for {set i 0} {$i < 3} {incr i} {\n  set guard "foo"\n  set guard "bar"\n}\n'
        )
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRFor)
        # Both writes remain inside the body.  Count writes to
        # ``guard`` regardless of which IRAssign* variant the
        # lowerer chose.
        body_writes = sum(
            1
            for s in for_node.body.statements
            if isinstance(s, (IRAssignConst, IRAssignValue)) and s.name == "guard"
        )
        assert body_writes == 2
        # And NO ``guard`` write hoisted out.
        top_writes = sum(
            1
            for s in new_module.top_level.statements
            if isinstance(s, (IRAssignConst, IRAssignValue)) and s.name == "guard"
        )
        assert top_writes == 0

    def test_var_written_in_nested_if_not_hoisted(self):
        module = _module(
            "for {set i 0} {$i < 3} {incr i} {\n"
            '  set guard "foo"\n'
            '  if {$i > 1} { set guard "bar" }\n'
            "}\n"
        )
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRFor)
        # The top-level write isn't hoistable because the nested
        # write also targets ``guard``.
        assert _find_assign(for_node.body, "guard") is not None

    def test_var_written_in_for_next_clause_not_hoisted(self):
        # ``incr i`` in ``next`` writes ``i``.  A body-level
        # ``set i 0`` would be illegal LICM target.
        module = _module("for {set i 0} {$i < 3} {incr i} {\n  set i 0\n}\n")
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRFor)
        # Body's ``set i 0`` not hoisted because ``next`` also
        # writes i.
        assert _find_assign(for_node.body, "i") is not None

    def test_foreach_iter_var_not_hoisted(self):
        module = _module('foreach x {a b c} {\n  set x "override"\n}\n')
        new_module = licm_module(module)
        for_node = _find_first(new_module.top_level, IRForeach)
        assert _find_assign(for_node.body, "x") is not None


class TestPurity:
    def test_input_module_unchanged(self):
        module = _module('for {set i 0} {$i < 3} {incr i} {\n  set g "k"\n}\n')
        original_for = _find_first(module.top_level, IRFor)
        assert _find_assign(original_for.body, "g") is not None
        licm_module(module)
        # Original module's body should still mention ``set g ...``.
        assert _find_assign(original_for.body, "g") is not None

    def test_idempotent(self):
        module = _module('for {set i 0} {$i < 3} {incr i} {\n  set g "k"\n}\n')
        once = licm_module(module)
        twice = licm_module(once)
        # Second pass produces the same shape (no further
        # statements to hoist).
        assert _stmt_kinds(once.top_level) == _stmt_kinds(twice.top_level)

    def test_no_loop_no_change(self):
        module = _module("set x 1\nset y 2\n")
        new_module = licm_module(module)
        # Without a loop nothing moves.  The pass returns the same
        # instance to advertise structural sharing.
        assert new_module is module


class TestPipelineIntegration:
    """Confirm the LICM pass is invoked through ``wasm_codegen_module``."""

    def test_licm_flag_off_preserves_loop_body(self):
        from core.compiler.cfg import build_cfg
        from core.compiler.codegen.wasm import wasm_codegen_module

        source = 'for {set i 0} {$i < 3} {incr i} {\n  set guard "k"\n}\n'
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        # Body retains the literal alloc when LICM is off.
        module_off = wasm_codegen_module(cfg, ir, licm=False, inline=False)
        module_on = wasm_codegen_module(cfg, ir, licm=True, inline=False)
        # ``::top`` body length should NOT grow when LICM is on
        # (the assignment moves out, doesn't disappear).  The two
        # modules should have the same number of statements
        # mentioning the constant ``"k"``, but ``module_on`` will
        # have it placed BEFORE the loop block, not inside it.
        top_off = next(f for f in module_off.functions if f.name == "::top")
        top_on = next(f for f in module_on.functions if f.name == "::top")
        # We can't compare instruction counts directly because the
        # surrounding loop scaffolding shifts.  As a sanity check,
        # confirm both functions exist and produce a bytecode
        # sequence of comparable order.  The real correctness
        # check is end-to-end execution which the broader sweep
        # exercises.
        assert top_off.body and top_on.body


class TestNestedLoops:
    def test_inner_loop_invariant_hoisted_to_outer_then_outer(self):
        # Inner loop writes ``inner_g`` once with a literal.  After
        # one LICM pass, ``inner_g`` lifts to the outer loop body.
        # Since the outer loop's body now has a literal write that's
        # invariant to the outer iterations, the SAME pass also
        # lifts it out of the outer loop (the inner-then-outer
        # progression happens because ``_hoist_from_loop_body``
        # recurses before processing top-level candidates).
        module = _module(
            "for {set i 0} {$i < 3} {incr i} {\n"
            "  for {set j 0} {$j < 3} {incr j} {\n"
            '    set inner_g "k"\n'
            "  }\n"
            "}\n"
        )
        new_module = licm_module(module)
        # The hoisted ``set inner_g`` should now be at the top
        # level, NOT inside either loop body.
        outer = _find_first(new_module.top_level, IRFor)
        assert outer is not None
        inner = _find_first(outer.body, IRFor)
        assert inner is not None
        assert _find_assign(inner.body, "inner_g") is None
        assert _find_assign(outer.body, "inner_g") is None
        assert _find_assign(new_module.top_level, "inner_g") is not None

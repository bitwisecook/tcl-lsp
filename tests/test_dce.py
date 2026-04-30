"""Tests for S5.4 dead-store elimination."""

from __future__ import annotations

import textwrap

from core.compiler.inlining import apply_inline_catalogue
from core.compiler.ir import IRAssignConst, IRAssignValue, IRIncr
from core.compiler.lowering import lower_to_ir
from core.compiler.passes.dce import dce_module
from core.compiler.var_escape import analyse_var_escape


def _prepare(source: str):
    """Lower + run the inline catalogue (which sets ``inline_decision``
    based on ``pure_leaf`` — DCE's eligibility gate)."""
    module = lower_to_ir(textwrap.dedent(source))
    summaries = analyse_var_escape(ir_module=module)
    return apply_inline_catalogue(module, summaries), summaries


def _proc_writes(module, qname: str, name: str) -> int:
    proc = module.procedures[qname]
    n = 0
    for s in proc.body.statements:
        if isinstance(s, (IRAssignConst, IRAssignValue, IRIncr)) and s.name == name:
            n += 1
    return n


class TestDeadStoreElimination:
    def test_unused_literal_store_removed(self):
        # ``set tmp 5`` is never read — pure dead store.  ``puts``
        # is splice-safe and the proc is pure_leaf, so DCE fires.
        module, _ = _prepare('proc f {} {\n  set tmp 5\n  puts "hi"\n}\n')
        assert _proc_writes(module, "::f", "tmp") == 1
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::f", "tmp") == 0

    def test_used_var_kept(self):
        # ``set x 5; puts $x`` — x is read, must stay.
        module, _ = _prepare("proc f {} {\n  set x 5\n  puts $x\n}\n")
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::f", "x") == 1

    def test_var_used_in_braced_substitution_kept(self):
        # ``${x}`` form should also keep ``x`` alive.
        module, _ = _prepare('proc f {} {\n  set x 5\n  puts "value=${x}"\n}\n')
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::f", "x") == 1

    def test_var_used_in_expr_kept(self):
        # ``expr {$x + 1}`` reads x via the expression AST.
        module, _ = _prepare("proc f {} {\n  set x 5\n  set y [expr {$x + 1}]\n  puts $y\n}\n")
        new_module = dce_module(module)
        # x is read via $x in the value — kept.
        assert _proc_writes(new_module, "::f", "x") == 1

    def test_multiple_writes_kept(self):
        # Multi-write case — DCE doesn't currently handle these
        # because the "only write" gate fails.  Both ``set tmp 5``
        # and ``set tmp 6`` stay.
        module, _ = _prepare('proc f {} {\n  set tmp 5\n  set tmp 6\n  puts "hi"\n}\n')
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::f", "tmp") == 2

    def test_non_pure_leaf_proc_untouched(self):
        # ``upvar`` makes the proc non-pure_leaf so DCE declines.
        # Even an obviously-dead store stays.
        module, _ = _prepare("proc f {name} {\n  upvar 1 $name v\n  set tmp 5\n  set v 10\n}\n")
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::f", "tmp") == 1

    def test_global_write_kept(self):
        # Writing to ``::result`` is observable from outside the
        # proc (other procs, the embedding host's eval calls).
        # DCE must NOT delete the write even if the proc body
        # never reads it back.
        module, _ = _prepare('proc f {} {\n  set ::result "hello"\n}\n')
        new_module = dce_module(module)
        # ``::result`` is a qualified global — preserved.
        proc = new_module.procedures["::f"]
        assert any(
            isinstance(s, (IRAssignConst, IRAssignValue)) and s.name == "::result"
            for s in proc.body.statements
        )

    def test_array_element_kept(self):
        # ``set arr(idx) value`` writes one element; other
        # elements of ``arr`` may be observable elsewhere.  DCE
        # declines.
        module, _ = _prepare('proc f {} {\n  set arr(x) 5\n  puts "hi"\n}\n')
        new_module = dce_module(module)
        proc = new_module.procedures["::f"]
        assert any(
            isinstance(s, (IRAssignConst, IRAssignValue)) and "(" in s.name
            for s in proc.body.statements
        )

    def test_param_write_kept(self):
        # Writing to a parameter slot — even if "unused" by the body
        # — must NOT be deleted.  The parameter's incoming value is
        # observable through ``[info args]`` / introspection.
        module, _ = _prepare('proc f {x} {\n  set x 5\n  puts "hi"\n}\n')
        new_module = dce_module(module)
        # The body's ``set x 5`` is the only write to x — but x is
        # a parameter, so DCE conservatively keeps it.
        assert _proc_writes(new_module, "::f", "x") == 1


class TestSideEffects:
    """Codex P1 (PR #237): DCE must not drop assignments whose RHS
    has observable side effects, even when the target is unread.
    A statement like ``set tmp [expr {[puts hi]}]`` writes ``tmp``
    *and* runs ``puts hi``; deleting it removes the user-visible
    output."""

    def test_iassign_value_with_command_subst_kept(self):
        # ``set tmp [some_cmd]`` — RHS is a command substitution.
        # tmp is unread by the rest of the body, but the command
        # has side effects.
        module, _ = _prepare("proc f {} {\n  set tmp [info commands]\n  puts ok\n}\n")
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::f", "tmp") == 1, (
            "set tmp [info commands] must stay — command-subst side effects "
            "are observable even though tmp is unread"
        )

    def test_iassign_expr_with_command_subst_kept(self):
        # ``set x [expr {[string length "hello"]}]`` — the expr
        # embeds a command substitution.  Removing the assignment
        # would remove the call.  ``_proc_writes`` doesn't count
        # IRAssignExpr (used in unrelated tests above), so we
        # inspect the body shape directly here.
        from core.compiler.ir import IRAssignExpr

        module, _ = _prepare(
            'proc f {} {\n  set x [expr {[string length "hello"]}]\n  puts ok\n}\n'
        )
        new_module = dce_module(module)
        proc = new_module.procedures["::f"]
        assigns = [s for s in proc.body.statements if isinstance(s, IRAssignExpr) and s.name == "x"]
        assert len(assigns) == 1, (
            "set x [expr {[...]}] must stay — the embedded command "
            "substitution has observable side effects even though x is unread"
        )

    def test_pure_literal_assign_still_removed(self):
        # ``set tmp 5`` has no side effects — DCE still fires on
        # the no-RHS-side-effect case (sanity check that the gate
        # didn't go too wide).
        module, _ = _prepare("proc f {} {\n  set tmp 5\n  puts ok\n}\n")
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::f", "tmp") == 0


class TestImplicitReturn:
    """PR #237 review: the value of a Tcl proc's last command is
    the proc's implicit return value.  ``proc p {} {set y 42}``
    returns ``42`` because ``set`` returns the value it stored.
    DCE must not delete the terminal statement even when the
    target is unread."""

    def test_terminal_set_kept_as_implicit_return(self):
        # Single-statement body: ``set y 42`` is both the only
        # write to y AND the proc's implicit return.  DCE must
        # keep it.
        module, _ = _prepare("proc p {} { set y 42 }\n")
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::p", "y") == 1, (
            "set y 42 must stay — it's the proc's implicit return value"
        )

    def test_non_terminal_dead_set_still_removed(self):
        # Earlier-statement dead store still gets removed.  The
        # terminal protection only covers the LAST statement.
        module, _ = _prepare("proc p {} {\n  set tmp 5\n  set y 42\n}\n")
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::p", "tmp") == 0
        # Terminal ``set y 42`` stays.
        assert _proc_writes(new_module, "::p", "y") == 1

    def test_terminal_set_with_side_effect_kept(self):
        # The new terminal-protection rule subsumes the existing
        # side-effect rule — both gates keep the assignment.
        module, _ = _prepare("proc p {} {\n  set y [info commands]\n}\n")
        new_module = dce_module(module)
        assert _proc_writes(new_module, "::p", "y") == 1


class TestNestedRecursion:
    """PR #237 review: DCE now recurses into nested control-flow
    bodies so dead stores inside ``if {…} { set y 42 }``,
    ``for { … } { … } {…}``, etc. get removed too."""

    def _proc_writes_recursive(self, module, qname: str, name: str) -> int:
        """Count writes to ``name`` anywhere in the proc body
        (top-level + every nested control-flow body)."""
        from core.compiler.ir import (
            IRAssignConst,
            IRAssignValue,
            IRBlock,
            IRCatch,
            IRFor,
            IRForeach,
            IRIf,
            IRIncr,
            IRSwitch,
            IRTry,
            IRUpFrame,
            IRWhile,
        )

        proc = module.procedures[qname]
        n = 0

        def walk(script):
            nonlocal n
            if script is None:
                return
            for s in script.statements:
                if isinstance(s, (IRAssignConst, IRAssignValue, IRIncr)) and s.name == name:
                    n += 1
                if isinstance(s, IRBlock):
                    walk(s.body)
                elif isinstance(s, IRIf):
                    for clause in s.clauses:
                        walk(clause.body)
                    walk(s.else_body)
                elif isinstance(s, IRFor):
                    walk(s.init)
                    walk(s.next)
                    walk(s.body)
                elif isinstance(s, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
                    walk(s.body)
                elif isinstance(s, IRTry):
                    walk(s.body)
                    for h in s.handlers:
                        walk(h.body)
                    walk(s.finally_body)
                elif isinstance(s, IRSwitch):
                    for arm in s.arms:
                        if arm.body is not None:
                            walk(arm.body)
                    walk(s.default_body)

        walk(proc.body)
        return n

    def test_dead_store_inside_if_clause_removed(self):
        # ``if {1} { set y 42 }`` — y is unread.  Pre-fix DCE only
        # walked top-level statements and missed this.
        module, _ = _prepare("proc f {} {\n  if {1} { set y 42 }\n  puts ok\n}\n")
        new_module = dce_module(module)
        assert self._proc_writes_recursive(new_module, "::f", "y") == 0

    def test_dead_store_inside_for_body_removed(self):
        module, _ = _prepare(
            "proc f {} {\n  for {set i 0} {$i < 3} {incr i} { set y 42 }\n  puts ok\n}\n"
        )
        new_module = dce_module(module)
        assert self._proc_writes_recursive(new_module, "::f", "y") == 0

    def test_used_var_inside_nested_if_kept(self):
        # ``set y 42`` inside an if AND ``puts $y`` outside —
        # y has a read so the write stays.
        module, _ = _prepare("proc f {} {\n  if {1} { set y 42 }\n  puts $y\n}\n")
        new_module = dce_module(module)
        assert self._proc_writes_recursive(new_module, "::f", "y") == 1


class TestPurity:
    def test_input_module_unchanged(self):
        module, _ = _prepare('proc f {} {\n  set tmp 5\n  puts "hi"\n}\n')
        original_writes = _proc_writes(module, "::f", "tmp")
        assert original_writes == 1
        dce_module(module)
        # Original module's body should still have the dead store.
        assert _proc_writes(module, "::f", "tmp") == 1

    def test_idempotent(self):
        module, _ = _prepare('proc f {} {\n  set tmp 5\n  puts "hi"\n}\n')
        once = dce_module(module)
        twice = dce_module(once)
        assert _proc_writes(once, "::f", "tmp") == _proc_writes(twice, "::f", "tmp")

    def test_no_change_when_no_dead_stores(self):
        module, _ = _prepare('proc f {} { puts "hi" }\n')
        new_module = dce_module(module)
        assert new_module is module

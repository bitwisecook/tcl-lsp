"""Tests for S5.4 GVN pass (`core.compiler.passes.gvn`)."""

from __future__ import annotations

import textwrap

from core.compiler.ir import IRAssignExpr, IRAssignValue
from core.compiler.lowering import lower_to_ir
from core.compiler.passes.gvn import gvn_module


def _module(source: str):
    return lower_to_ir(textwrap.dedent(source))


def _proc_assign_kinds(module, qname: str, name: str):
    """Return the IR-stmt class name for each write to ``name``
    inside the proc's body, in source order."""
    proc = module.procedures[qname]
    out = []
    for s in proc.body.statements:
        if isinstance(s, (IRAssignExpr, IRAssignValue)) and s.name == name:
            out.append(type(s).__name__)
    return out


class TestRedundantExprElimination:
    def test_two_identical_expr_collapses_second(self):
        # ``set y [expr {$x + 1}]; set z [expr {$x + 1}]`` —
        # the second computation is redundant.  GVN replaces it
        # with ``set z $y``.
        module = _module(
            "proc f {x} {\n  set y [expr {$x + 1}]\n  set z [expr {$x + 1}]\n  return $z\n}\n"
        )
        new_module = gvn_module(module)
        assert _proc_assign_kinds(new_module, "::f", "y") == ["IRAssignExpr"]
        assert _proc_assign_kinds(new_module, "::f", "z") == ["IRAssignValue"]
        proc = new_module.procedures["::f"]
        z_stmt = next(
            s for s in proc.body.statements if isinstance(s, IRAssignValue) and s.name == "z"
        )
        assert z_stmt.value == "$y"

    def test_intervening_write_invalidates_table(self):
        # ``set y [expr {$x + 1}]; set x 99; set z [expr {$x +
        # 1}]`` — the source var ``x`` was rebound, so the second
        # expr is NOT equivalent.  GVN keeps both IRAssignExpr.
        module = _module(
            "proc f {x} {\n"
            "  set y [expr {$x + 1}]\n"
            "  set x 99\n"
            "  set z [expr {$x + 1}]\n"
            "  return $z\n"
            "}\n"
        )
        new_module = gvn_module(module)
        assert _proc_assign_kinds(new_module, "::f", "z") == ["IRAssignExpr"]

    def test_distinct_expressions_kept(self):
        module = _module(
            "proc f {x} {\n  set y [expr {$x + 1}]\n  set z [expr {$x + 2}]\n  return $z\n}\n"
        )
        new_module = gvn_module(module)
        assert _proc_assign_kinds(new_module, "::f", "z") == ["IRAssignExpr"]

    def test_expr_with_command_subst_not_cached(self):
        # ``[cmd]`` has unknown side effects — GVN must not
        # cache its result.  Both writes stay.
        module = _module(
            "proc f {x} {\n  set y [expr {$x + [foo]}]\n  set z [expr {$x + [foo]}]\n}\n"
        )
        new_module = gvn_module(module)
        assert _proc_assign_kinds(new_module, "::f", "z") == ["IRAssignExpr"]

    def test_intervening_call_clears_table(self):
        # An ``IRCall`` between the two expressions might side-
        # effect via eval / upvar — clear the table to be safe.
        module = _module(
            'proc f {x} {\n  set y [expr {$x + 1}]\n  puts "between"\n  set z [expr {$x + 1}]\n}\n'
        )
        new_module = gvn_module(module)
        assert _proc_assign_kinds(new_module, "::f", "z") == ["IRAssignExpr"]

    def test_rand_expr_call_not_cached(self):
        # Codex P2 (PR #237): ``rand()`` is non-deterministic —
        # caching it would collapse two distinct evaluations into
        # the same value, changing program output.  Both writes
        # must stay as ``IRAssignExpr``.
        module = _module("proc f {} {\n  set a [expr {rand()}]\n  set b [expr {rand()}]\n}\n")
        new_module = gvn_module(module)
        assert _proc_assign_kinds(new_module, "::f", "b") == ["IRAssignExpr"]

    def test_pure_expr_call_still_cached(self):
        # Sanity: pure math functions like ``abs()`` ARE safely
        # cacheable — same argument always returns the same result.
        module = _module("proc f {x} {\n  set a [expr {abs($x)}]\n  set b [expr {abs($x)}]\n}\n")
        new_module = gvn_module(module)
        assert _proc_assign_kinds(new_module, "::f", "b") == ["IRAssignValue"]


class TestPurity:
    def test_idempotent(self):
        module = _module("proc f {x} {\n  set y [expr {$x + 1}]\n  set z [expr {$x + 1}]\n}\n")
        once = gvn_module(module)
        twice = gvn_module(once)
        assert _proc_assign_kinds(once, "::f", "z") == _proc_assign_kinds(twice, "::f", "z")

    def test_no_change_when_no_duplicates(self):
        module = _module("proc f {x} { set y [expr {$x + 1}] }\n")
        new_module = gvn_module(module)
        assert new_module is module

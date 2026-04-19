"""Tests for IRBarrier relaxation on static ``eval`` / ``uplevel`` bodies.

Ensures the lowering dispatcher produces :class:`IRBlock` /
:class:`IRUpFrame` for the recognised braced-literal shapes, and
keeps :class:`IRBarrier` for every dynamic shape (``$body``
reference, ``[cmd]`` substitution, dynamic level, nested dynamic
barrier).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.ir import IRBarrier, IRBlock, IRUpFrame
from core.compiler.lowering import lower_to_ir


def _first_stmt(source: str):
    mod = lower_to_ir(source)
    stmts = mod.top_level.statements
    assert stmts, "expected at least one top-level statement"
    return stmts[0]


class TestEvalRelaxation:
    def test_static_body_lowers_to_irblock(self):
        stmt = _first_stmt("eval {set x 1}")
        assert isinstance(stmt, IRBlock)
        assert len(stmt.body.statements) == 1
        # source_tokens argv[0] discriminates eval-shape from
        # namespace-eval-shape for VM codegen.
        assert stmt.source_tokens is not None
        assert stmt.source_tokens.argv_texts[0] == "eval"

    def test_substituted_body_stays_barrier(self):
        mod = lower_to_ir("set body {set x 1}\neval $body")
        assert any(isinstance(s, IRBarrier) for s in mod.top_level.statements)

    def test_list_literal_command_subst_relaxes(self):
        # ``eval [list set x 1]`` — the inner ``list`` with all-literal
        # args is statically decidable, so the outer eval relaxes to
        # IRBlock with the synthesised body ``set x 1``.
        stmt = _first_stmt("eval [list set x 1]")
        assert isinstance(stmt, IRBlock)

    def test_non_list_command_subst_stays_barrier(self):
        # ``eval [foo ...]`` — ``foo`` is not ``list``, so the body
        # cannot be synthesised statically.
        stmt = _first_stmt("eval [foo a b]")
        assert isinstance(stmt, IRBarrier)

    def test_list_with_var_subst_stays_barrier(self):
        # ``eval [list set x $v]`` — ``$v`` is a dynamic substitution,
        # so the body cannot be synthesised statically.
        mod = lower_to_ir("set v 1\neval [list set x $v]")
        assert any(isinstance(s, IRBarrier) for s in mod.top_level.statements)

    def test_multi_arg_eval_stays_barrier(self):
        stmt = _first_stmt("eval set x 1")
        assert isinstance(stmt, IRBarrier)

    def test_nested_dynamic_barrier_poisons_relaxation(self):
        # Braced body, but inside is an eval of a $var — the nested
        # barrier has dynamic shape, so the whole outer relax is
        # forbidden.
        stmt = _first_stmt("eval {eval $body}")
        assert isinstance(stmt, IRBarrier)

    def test_nested_static_barrier_relaxes(self):
        # Nested eval with a braced body is itself statically
        # decidable — outer relax proceeds.
        stmt = _first_stmt("eval {eval {set x 1}}")
        assert isinstance(stmt, IRBlock)


class TestUplevelRelaxation:
    def test_level_one_body_lowers_to_upframe(self):
        stmt = _first_stmt("uplevel 1 {set x 1}")
        assert isinstance(stmt, IRUpFrame)
        assert stmt.frame_shift == 1
        assert len(stmt.body.statements) == 1

    def test_hash_zero_lowers_to_upframe_with_large_shift(self):
        stmt = _first_stmt("uplevel #0 {set ::g 3}")
        assert isinstance(stmt, IRUpFrame)
        assert stmt.frame_shift == 0x3FFF_FFFF

    def test_implicit_level_body_lowers_to_upframe(self):
        # ``uplevel`` without an explicit level defaults to 1.
        stmt = _first_stmt("uplevel {set x 1}")
        assert isinstance(stmt, IRUpFrame)
        assert stmt.frame_shift == 1

    def test_dynamic_level_stays_barrier(self):
        mod = lower_to_ir("set lvl 1\nuplevel $lvl {set x 1}")
        assert any(isinstance(s, IRBarrier) for s in mod.top_level.statements)

    def test_dynamic_body_stays_barrier(self):
        mod = lower_to_ir("set body {set x 1}\nuplevel 1 $body")
        assert any(isinstance(s, IRBarrier) for s in mod.top_level.statements)

    def test_nested_dynamic_barrier_inside_body_poisons_relaxation(self):
        stmt = _first_stmt("uplevel 1 {eval $body}")
        assert isinstance(stmt, IRBarrier)

    def test_bare_integer_level_two_lowers(self):
        stmt = _first_stmt("uplevel 2 {set x 1}")
        assert isinstance(stmt, IRUpFrame)
        assert stmt.frame_shift == 2


class TestBarrierGateSafety:
    def test_upvar_still_produces_ircall_not_barrier(self):
        # ``upvar`` has a dedicated lowering hook that returns IRCall
        # with defs — barrier relaxation must not interfere.  Inspect
        # the proc's body directly since the top-level statement is
        # the proc definition, registered on the module.
        mod = lower_to_ir("proc p {} { upvar 1 x y }")
        assert "::p" in mod.procedures
        body = mod.procedures["::p"].body
        assert body.statements
        first = body.statements[0]
        assert not isinstance(first, IRBarrier)

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

from compiler.ir import IRBarrier, IRBlock, IRUpFrame
from compiler.lowering import lower_to_ir


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

    def test_list_with_whitespace_value_stays_barrier(self):
        # ``eval [list puts "a b"]`` — synthesising ``puts a b`` as
        # the body would reparse as two args (``a``, ``b``) instead
        # of one (``a b``).  Refuse to relax rather than produce a
        # body with different argument-structure from the source.
        stmt = _first_stmt('eval [list puts "a b"]')
        assert isinstance(stmt, IRBarrier)

    def test_list_with_semicolon_stays_barrier(self):
        # ``;`` is a command separator in Tcl; a bareword containing
        # one would split the synthesised body into multiple
        # commands.  Stay conservative.
        stmt = _first_stmt('eval [list puts ";"]')
        assert isinstance(stmt, IRBarrier)

    def test_list_with_backslash_stays_barrier(self):
        # A backslash escape inside a bareword needs canonical list
        # quoting to round-trip correctly.  Refuse to relax.
        stmt = _first_stmt("eval [list puts a\\nb]")
        assert isinstance(stmt, IRBarrier)

    def test_list_with_leading_hash_stays_barrier(self):
        # ``#`` at the start of a list element needs brace-wrap in
        # canonical Tcl list format so the reparse treats it as an
        # element rather than a comment.  Refuse to relax.
        stmt = _first_stmt("eval [list set x #hash]")
        assert isinstance(stmt, IRBarrier)

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

    def test_multi_word_body_stays_barrier(self):
        # ``uplevel 1 puts hello`` — Tcl concatenates the words
        # after the level into a single script ``puts hello``.
        # Synthesising that concat at compile time requires
        # per-token list-quoting we don't reproduce; bail rather
        # than relax only the last word (which would drop ``puts``).
        stmt = _first_stmt("uplevel 1 puts hello")
        assert isinstance(stmt, IRBarrier)

    def test_multi_word_body_no_level_stays_barrier(self):
        # ``uplevel puts hello`` — default level, two body words.
        # Same concat issue.
        stmt = _first_stmt("uplevel puts hello")
        assert isinstance(stmt, IRBarrier)


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


class TestConstMapBarrierRelaxation:
    """Const-propagate ``set body {literal}`` through ``$body`` at
    the ``uplevel`` / ``eval`` barrier-relaxation gate."""

    def test_proc_set_body_then_uplevel_relaxes(self):
        mod = lower_to_ir("proc p {} { set body {set x 1}; uplevel 1 $body }")
        stmts = mod.procedures["::p"].body.statements
        # IRAssignConst, then IRUpFrame with the body folded in.
        assert any(isinstance(s, IRUpFrame) for s in stmts)

    def test_proc_set_body_then_eval_relaxes(self):
        mod = lower_to_ir("proc p {} { set body {set x 1}; eval $body }")
        stmts = mod.procedures["::p"].body.statements
        assert any(isinstance(s, IRBlock) for s in stmts)

    def test_reassign_invalidates_const_map(self):
        mod = lower_to_ir("proc p {dyn} { set body {set x 1}; set body $dyn; uplevel 1 $body }")
        stmts = mod.procedures["::p"].body.statements
        # Re-assignment to ``body`` with a dynamic RHS drops the map
        # entry — subsequent uplevel cannot relax.
        assert any(isinstance(s, IRBarrier) and s.command == "uplevel" for s in stmts)

    def test_control_flow_invalidates_const_map(self):
        # ``if`` may write to arbitrary vars in its body; conservatively
        # drop the whole map so ``uplevel 1 $body`` after it stays a
        # barrier.
        mod = lower_to_ir(
            "proc p {c} { set body {set x 1}; if {$c} { set body {set x 2} }; uplevel 1 $body }"
        )
        stmts = mod.procedures["::p"].body.statements
        assert any(isinstance(s, IRBarrier) and s.command == "uplevel" for s in stmts)

    def test_top_level_const_map_stays_disabled(self):
        # Top-level ``set body {…}`` creates a global; const-propagating
        # it is unsound because other code could write the global.
        mod = lower_to_ir("set body {set x 1}\nuplevel 1 $body")
        assert any(isinstance(s, IRBarrier) for s in mod.top_level.statements)

    def test_param_not_in_const_map(self):
        # A proc parameter is not a ``set`` assignment — it never
        # enters the const-map, so ``uplevel 1 $param`` stays a
        # barrier (the passthrough-inlining pass handles the
        # single-param shape separately at call sites).
        mod = lower_to_ir("proc p {body} { uplevel 1 $body }")
        stmts = mod.procedures["::p"].body.statements
        assert any(isinstance(s, IRBarrier) for s in stmts)

    def test_namespace_qualified_name_not_tracked(self):
        # ``set ::body {...}`` is a global write; don't track it.
        mod = lower_to_ir("proc p {} { set ::body {set x 1}; uplevel 1 $body }")
        stmts = mod.procedures["::p"].body.statements
        assert any(isinstance(s, IRBarrier) and s.command == "uplevel" for s in stmts)

    def test_array_ref_not_tracked(self):
        mod = lower_to_ir("proc p {} { set arr(k) {set x 1}; uplevel 1 $body }")
        stmts = mod.procedures["::p"].body.statements
        assert any(isinstance(s, IRBarrier) and s.command == "uplevel" for s in stmts)

    def test_nested_dynamic_barrier_in_literal_still_poisons(self):
        mod = lower_to_ir("proc p {} { set body {eval $x}; uplevel 1 $body }")
        stmts = mod.procedures["::p"].body.statements
        assert any(isinstance(s, IRBarrier) and s.command == "uplevel" for s in stmts)

    def test_catch_body_inherits_outer_const_map(self):
        # ``set body {literal}; catch {uplevel 1 $body}`` — the catch's
        # inner scope should inherit the outer proc's const-map so the
        # nested uplevel can relax.  Mirrors tcltest's
        # ``catch {uplevel 1 $setup}`` shape where ``setup`` is set to
        # a literal earlier in the proc.
        from compiler.ir import IRCatch

        mod = lower_to_ir("proc p {} { set body {set x 1}; catch {uplevel 1 $body} err }")
        stmts = mod.procedures["::p"].body.statements
        catch_stmts = [s for s in stmts if isinstance(s, IRCatch)]
        assert catch_stmts
        inner = catch_stmts[0].body.statements
        assert any(isinstance(s, IRUpFrame) for s in inner)

    def test_if_body_inherits_outer_const_map(self):
        from compiler.ir import IRIf

        mod = lower_to_ir("proc p {c} { set body {set x 1}; if {$c} { uplevel 1 $body } }")
        stmts = mod.procedures["::p"].body.statements
        if_stmts = [s for s in stmts if isinstance(s, IRIf)]
        assert if_stmts
        inner = if_stmts[0].clauses[0].body.statements
        assert any(isinstance(s, IRUpFrame) for s in inner)

    def test_inner_reassignment_does_not_leak_to_parent(self):
        # Child scope re-assigns ``body`` to something dynamic — the
        # child's local const-map loses the entry, but the parent's
        # map is only cleared because ``if`` itself is structured IR.
        # The outer uplevel stays a barrier (the parent's invalidation
        # is still in charge post-``if``).
        mod = lower_to_ir("proc p {c} { set body {a}; if {$c} { set body $c } ; uplevel 1 $body }")
        stmts = mod.procedures["::p"].body.statements
        assert any(isinstance(s, IRBarrier) and s.command == "uplevel" for s in stmts)

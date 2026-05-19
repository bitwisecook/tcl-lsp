"""Tests for SSA construction."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser.semantic_model import Range
from compiler.cfg import CFGBlock, CFGBranch, CFGFunction, CFGGoto, CFGReturn, build_cfg
from compiler.expr_ast import ExprRaw
from compiler.ir import IRAssignConst, IRAssignExpr
from compiler.lowering import lower_to_ir
from compiler.parsing.tokens import SourcePosition
from compiler.ssa import build_ssa


def _dummy_range() -> Range:
    p = SourcePosition(line=0, character=0, offset=0)
    return Range(start=p, end=p)


class TestSSA:
    def test_linear_versions_increment(self):
        mod = lower_to_ir("set a 1\nset a 2")
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)
        entry_block = ssa.blocks[ssa.entry]
        assert entry_block.exit_versions.get("a") == 2
        assert len(entry_block.statements) == 2
        assert entry_block.statements[0].defs.get("a") == 1
        assert entry_block.statements[1].defs.get("a") == 2

    def test_if_merge_creates_phi(self):
        source = "if {$x} {set a 1} else {set a 2}\nset b [expr {$a + 0}]"
        mod = lower_to_ir(source)
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)

        has_phi_for_a = any(
            any(phi.name == "a" for phi in block.phis) for block in ssa.blocks.values()
        )
        assert has_phi_for_a

        uses_a_phi_version = any(
            any(stmt.uses.get("a", 0) > 0 for stmt in block.statements)
            for block in ssa.blocks.values()
        )
        assert uses_a_phi_version

    def test_dominator_metadata_present(self):
        mod = lower_to_ir("set a 1\nif {$a} {set b 2}\nset c 3")
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)
        assert ssa.entry in ssa.idom
        assert ssa.entry in ssa.dominator_tree
        assert ssa.entry in ssa.dominance_frontier

    def test_incr_reads_previous_version(self):
        mod = lower_to_ir("set a 1\nincr a\nputs $a")
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)
        entry_block = ssa.blocks[ssa.entry]

        assert len(entry_block.statements) == 3
        incr_stmt = entry_block.statements[1]
        assert incr_stmt.uses.get("a") == 1
        assert incr_stmt.defs.get("a") == 2
        assert entry_block.statements[2].uses.get("a") == 2

    def test_loop_cfg_phi_via_dominance_frontier(self):
        r = _dummy_range()
        cfg = CFGFunction(
            name="::loop_test",
            entry="entry",
            blocks={
                "entry": CFGBlock(
                    name="entry",
                    statements=(IRAssignConst(range=r, name="a", value="0"),),
                    terminator=CFGGoto(target="header", range=r),
                ),
                "header": CFGBlock(
                    name="header",
                    statements=(),
                    terminator=CFGBranch(
                        condition=ExprRaw(text="$cond"),
                        true_target="body",
                        false_target="exit",
                        range=r,
                    ),
                ),
                "body": CFGBlock(
                    name="body",
                    statements=(IRAssignExpr(range=r, name="a", expr=ExprRaw(text="$a + 1")),),
                    terminator=CFGGoto(target="header", range=r),
                ),
                "exit": CFGBlock(
                    name="exit",
                    statements=(),
                    terminator=CFGReturn(value="$a", range=r),
                ),
            },
        )
        ssa = build_ssa(cfg)
        header = ssa.blocks["header"]
        assert any(phi.name == "a" for phi in header.phis)
        a_phi = [phi for phi in header.phis if phi.name == "a"][0]
        assert "entry" in a_phi.incoming
        assert "body" in a_phi.incoming


class TestBarrierVarWriteRoles:
    """Issue #249 — registry-driven ``ArgRole.VAR_WRITE`` def-tracking on barriers.

    ``trace add variable`` registers a callback that may rewrite the
    target at runtime, so SSA must see *name* as a definition site.
    """

    def test_trace_add_variable_marks_name_as_def(self):
        # ``trace add variable x ...`` becomes an IRBarrier.  SSA must
        # see ``x`` as defined so a later read is the trace's version,
        # not a free use.
        mod = lower_to_ir("trace add variable x write watcher\nset y $x")
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)
        entry = ssa.blocks[ssa.entry]
        assert entry.exit_versions.get("x") == 1

    def test_scope_alias_barriers_skipped_at_def_layer(self):
        # ``global``/``variable``/``upvar`` carry ``VAR_WRITE`` on arg 0
        # too, but the static role only marks the first position — for
        # vararg forms like ``global x y z`` that would silently miss
        # ``y`` and ``z``.  In practice these commands have specific
        # lowering hooks that emit an ``IRCall`` with the full ``defs``
        # tuple; SSA's barrier path skips the registry ``VAR_WRITE``
        # query for scope-alias commands as a safety net so any future
        # spec or lowering edit can't silently regress to partial defs.
        from compiler.ir import IRBarrier
        from compiler.ssa import _defs

        r = _dummy_range()
        for cmd in ("global", "variable"):
            barrier = IRBarrier(
                range=r, reason="dynamic command", command=cmd, args=("x", "y", "z")
            )
            assert _defs(barrier) == (), cmd

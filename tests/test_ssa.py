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
from compiler.ssa import (
    _compute_idom_fast,
    _dominators,
    _immediate_dominators,
    _predecessors,
    _reachable_blocks,
    build_ssa,
)
from shared.tokens import SourcePosition


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


class TestSharedTraversal:
    """The centralised reverse-postorder + Cooper-Harvey-Kennedy idom."""

    _SOURCES = [
        "set a 1\nset b 2",
        "if {$x} {set a 1} else {set a 2}\nset b [expr {$a + 0}]",
        "set i 0\nwhile {$i < 10} {set i [expr {$i + 1}]}\nset done 1",
        "for {set i 0} {$i < 5} {incr i} {if {$i} {set a 1} else {set a 2}}",
        "switch $x {a {set r 1} b {set r 2} default {set r 3}}\nset y $r",
        "if {$a} {if {$b} {set c 1} else {set c 2}} else {set c 3}\nset d $c",
        "while {$a} {while {$b} {set x 1}\nset y 2}\nset z 3",
    ]

    def _cfgs(self):
        for src in self._SOURCES:
            yield build_cfg(lower_to_ir(src)).top_level

    def test_reverse_postorder_entry_first_and_complete(self):
        for cfg in self._cfgs():
            order = cfg.reverse_postorder()
            # Same set as cfg.blocks (reachable RPO + unreachable tail).
            assert set(order) == set(cfg.blocks)
            assert len(order) == len(cfg.blocks)
            assert order[0] == cfg.entry

    def test_reverse_postorder_respects_non_back_edges(self):
        # A valid RPO places every block before all of its successors
        # except across *back edges* (loop-closing edges).  In a reducible
        # CFG a back edge is exactly one whose target dominates its source,
        # so identify back edges via the dominance relation and require
        # strict predecessor-before-successor ordering for every other edge.
        from compiler.cfg import _block_successors

        for cfg in self._cfgs():
            order = cfg.reverse_postorder()
            pos = {bn: i for i, bn in enumerate(order)}
            reachable = _reachable_blocks(cfg)
            preds = _predecessors(cfg)
            dom = _dominators(cfg, reachable, preds)
            for bn in reachable:
                for succ in _block_successors(cfg.blocks[bn].terminator):
                    if succ not in reachable:
                        continue
                    # Back edge (succ dominates bn) closes a loop, so RPO is
                    # permitted to place succ earlier.  Every non-back edge
                    # must order the predecessor strictly before the successor.
                    if succ in dom[bn]:
                        continue
                    assert pos[bn] < pos[succ], (bn, succ, order)

    def test_compute_idom_fast_matches_reference(self):
        # The production CHK path must produce exactly the same idom map
        # as the naive set-based reference (_dominators →
        # _immediate_dominators) on every shape.
        for cfg in self._cfgs():
            reachable = _reachable_blocks(cfg)
            preds = _predecessors(cfg)
            dom = _dominators(cfg, reachable, preds)
            reference = _immediate_dominators(cfg, reachable, dom)
            fast = _compute_idom_fast(cfg, reachable, preds)
            assert fast == reference


class TestDeepCFGStackSafety:
    """A long block chain must not overflow the worker-thread stack.

    The recursive dominator-tree rename / DFS traversal blew the stack on
    machine-generated procs that lower to thousands of sequential blocks
    (e.g. ~700 chained ``if {$c == N} {return X}`` arms).  Both the shared
    reverse_postorder and the SSA rename walk are now iterative, so this
    must complete under the common 2 MB worker-thread stack.
    """

    def _deep_chain_cfg(self, n: int) -> CFGFunction:
        # entry -> b1 -> b2 -> ... -> bn -> exit, each block a trivial set.
        r = _dummy_range()
        blocks: dict[str, CFGBlock] = {}
        names = ["entry"] + [f"b{i}" for i in range(1, n + 1)]
        for idx, name in enumerate(names):
            stmt = IRAssignConst(range=r, name=f"v{idx}", value="1")
            nxt = names[idx + 1] if idx + 1 < len(names) else None
            term = CFGGoto(target=nxt, range=r) if nxt else CFGReturn(range=r)
            blocks[name] = CFGBlock(name=name, statements=(stmt,), terminator=term)
        return CFGFunction(name="deep", entry="entry", blocks=blocks)

    def test_deep_chain_under_small_stack(self):
        import threading

        cfg = self._deep_chain_cfg(8000)

        result: dict[str, object] = {}

        def run() -> None:
            try:
                ssa = build_ssa(cfg)
                # idom forms a chain: each block's idom is its predecessor.
                result["ok"] = len(ssa.blocks) == len(cfg.blocks)
                result["rpo_len"] = len(cfg.reverse_postorder())
            except Exception as exc:  # pragma: no cover - failure path
                result["error"] = repr(exc)

        old = threading.stack_size(2 * 1024 * 1024)
        try:
            t = threading.Thread(target=run)
            t.start()
            t.join()
        finally:
            threading.stack_size(old)

        assert "error" not in result, result.get("error")
        assert result.get("ok") is True
        assert result.get("rpo_len") == len(cfg.blocks)

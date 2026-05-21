"""Tests for def-use chain construction."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.cfg import build_cfg
from core.compiler.def_use import DefKind, UseKind, build_def_use_chains
from core.compiler.lowering import lower_to_ir
from core.compiler.ssa import build_ssa


def _build(source: str):
    """Helper: source → SSA → def-use chains."""
    mod = lower_to_ir(source)
    cfg = build_cfg(mod).top_level
    ssa = build_ssa(cfg)
    return build_def_use_chains(ssa, cfg=cfg), ssa


class TestDefUseBasic:
    def test_linear_single_use(self):
        du, _ = _build("set x 1\nset y [expr {$x + 1}]")
        # x#1 should have exactly 1 use (in set y)
        x_chain = du.chain_for("x", 1)
        assert x_chain is not None
        assert x_chain.use_count == 1
        assert x_chain.uses[0].kind is UseKind.OPERAND

    def test_dead_store(self):
        du, _ = _build("set x 1\nset x 2")
        # x#1 should be dead (overwritten immediately)
        x1 = du.chain_for("x", 1)
        assert x1 is not None
        assert x1.is_dead

        # x#2 should also be dead (never used)
        x2 = du.chain_for("x", 2)
        assert x2 is not None
        assert x2.is_dead

    def test_multiple_uses(self):
        du, _ = _build("set x 1\nset y $x\nset z $x")
        x1 = du.chain_for("x", 1)
        assert x1 is not None
        assert x1.use_count == 2

    def test_reaching_defs(self):
        du, _ = _build("set x 1\nset x 2\nset x 3")
        defs = du.reaching_defs("x")
        assert len(defs) == 3

    def test_dead_chains_property(self):
        du, _ = _build("set x 1\nset y 2")
        dead = du.dead_chains
        assert len(dead) == 2  # both x and y are unused

    def test_total_counts(self):
        du, _ = _build("set x 1\nset y [expr {$x + 1}]")
        assert du.total_defs == 2
        assert du.total_uses == 1


class TestDefUsePhi:
    def test_if_merge_creates_phi_chain(self):
        source = "if {$cond} {set a 1} else {set a 2}\nset b [expr {$a + 0}]"
        du, ssa = _build(source)

        # Find phi definition of 'a'
        phi_defs = [
            key
            for key in du.chains
            if key[0] == "a" and du.chains[key].definition.kind is DefKind.PHI
        ]
        assert len(phi_defs) >= 1, "Expected at least one phi def for 'a'"

        # The phi chain should have uses (from 'set b ...')
        phi_chain = du.chains[phi_defs[0]]
        assert phi_chain.use_count >= 1

    def test_phi_incoming_edges_are_uses(self):
        source = "if {$cond} {set a 1} else {set a 2}"
        du, _ = _build(source)

        # a#1 and a#2 should have PHI_INCOMING uses
        a_defs = du.reaching_defs("a")
        phi_uses = []
        for key in a_defs:
            chain = du.chains[key]
            for use in chain.uses:
                if use.kind is UseKind.PHI_INCOMING:
                    phi_uses.append(use)

        assert len(phi_uses) >= 2, "Expected phi incoming uses for both branches"


class TestDefUseTerminator:
    def test_branch_condition_is_use(self):
        du, _ = _build("set cond 1\nif {$cond} { set x 1 }")
        cond1 = du.chain_for("cond", 1)
        assert cond1 is not None
        assert not cond1.is_dead, "cond#1 is read by branch — should not be dead"
        assert cond1.use_count >= 1
        has_terminator_use = any(u.kind is UseKind.TERMINATOR for u in cond1.uses)
        assert has_terminator_use

    def test_while_condition_is_use(self):
        du, _ = _build("set i 0\nwhile {$i < 10} {incr i}")
        # The phi-merged version of i is used in the while condition
        i_defs = du.reaching_defs("i")
        has_terminator = False
        for key in i_defs:
            chain = du.chains[key]
            for use in chain.uses:
                if use.kind is UseKind.TERMINATOR:
                    has_terminator = True
        assert has_terminator, "Expected at least one TERMINATOR use for loop condition"


class TestDefUseLoop:
    def test_loop_phi(self):
        source = "set i 0\nwhile {$i < 10} {incr i}"
        du, _ = _build(source)

        # Should have defs for i (initial, phi, incr)
        i_defs = du.reaching_defs("i")
        assert len(i_defs) >= 2, f"Expected ≥2 defs for i, got {len(i_defs)}"


class TestDefUseProc:
    def test_proc_parameters(self):
        source = "proc foo {x y} { set z [expr {$x + $y}]\n return $z }"
        mod = lower_to_ir(source)
        cfg_mod = build_cfg(mod)
        for qname, cfg in cfg_mod.procedures.items():
            ssa = build_ssa(cfg)
            du = build_def_use_chains(ssa, cfg=cfg)
            # Parameters should appear as version-0 chains
            # and have uses from the expr
            x_uses = du.uses_of("x", 0)
            y_uses = du.uses_of("y", 0)
            # Each parameter is read exactly once, in the ``$x + $y`` expr.
            assert len(x_uses) == 1
            assert len(y_uses) == 1


class TestDefUseResultMethods:
    def test_is_dead_method(self):
        du, _ = _build("set x 1\nset y $x")
        assert du.is_dead("x", 1) is False
        assert du.is_dead("y", 1) is True

    def test_is_dead_nonexistent_key(self):
        du, _ = _build("set x 1")
        assert du.is_dead("nonexistent", 99) is True

    def test_chain_for_returns_none(self):
        du, _ = _build("set x 1")
        assert du.chain_for("nonexistent", 99) is None

    def test_has_phi_use(self):
        source = "if {$cond} {set a 1} else {set a 2}"
        du, _ = _build(source)
        # At least one branch definition should have a phi use
        a_defs = du.reaching_defs("a")
        has_phi = any(du.chains[k].has_phi_use for k in a_defs)
        assert has_phi


class TestDefUseNestedControlFlow:
    def test_if_inside_while(self):
        source = "set i 0\nwhile {$i < 10} {\nif {$i > 5} {set x 1} else {set x 2}\nincr i\n}"
        du, _ = _build(source)
        i_defs = du.reaching_defs("i")
        assert len(i_defs) >= 2
        x_defs = du.reaching_defs("x")
        assert len(x_defs) >= 2

    def test_foreach_loop(self):
        source = "foreach item {a b c} { set result $item }"
        du, _ = _build(source)
        item_defs = du.reaching_defs("item")
        assert len(item_defs) >= 1

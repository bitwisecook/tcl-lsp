"""Tests for CFG construction from IR."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.cfg import CFGBranch, CFGGoto, CFGReturn, build_cfg
from core.compiler.expr_ast import expr_text
from core.compiler.ir import IRAssignConst
from core.compiler.lowering import lower_to_ir


class TestCFG:
    def test_linear_script_cfg(self):
        mod = lower_to_ir("set a 1\nset b 2")
        cfg = build_cfg(mod).top_level
        entry = cfg.blocks[cfg.entry]
        assert len(entry.statements) == 2
        assert isinstance(entry.statements[0], IRAssignConst)
        assert isinstance(entry.terminator, CFGGoto)

    def test_if_creates_branching_cfg(self):
        mod = lower_to_ir("if {$x > 0} {set y 1} else {set y 0}\nset z 1")
        cfg = build_cfg(mod).top_level
        assert any(isinstance(b.terminator, CFGBranch) for b in cfg.blocks.values())
        has_z = any(
            any(isinstance(s, IRAssignConst) and s.name == "z" for s in b.statements)
            for b in cfg.blocks.values()
        )
        assert has_z

    def test_switch_creates_dispatch_branches(self):
        mod = lower_to_ir("switch $x {a {set y 1} b - default {set y 0}}")
        cfg = build_cfg(mod).top_level
        branch_count = sum(1 for b in cfg.blocks.values() if isinstance(b.terminator, CFGBranch))
        assert branch_count >= 2

    def test_for_creates_loop_cfg(self):
        mod = lower_to_ir("for {set i 0} {$i < 3} {incr i} {set sum [expr {$sum + $i}]}")
        cfg = build_cfg(mod).top_level

        for_headers = [
            name
            for name, block in cfg.blocks.items()
            if isinstance(block.terminator, CFGBranch) and "for_header" in name
        ]
        assert for_headers
        header = for_headers[0]
        term = cfg.blocks[header].terminator
        assert isinstance(term, CFGBranch)
        assert "i < 3" in expr_text(term.condition)
        assert term.true_target in cfg.blocks
        assert term.false_target in cfg.blocks
        assert term.false_target in cfg.loop_nodes

    def test_return_terminates_block(self):
        source = textwrap.dedent("""\
            proc foo {} {
                set x 1
                return $x
                set y 2
            }
        """)
        mod = lower_to_ir(source)
        cfgm = build_cfg(mod)
        proc_cfg = cfgm.procedures["::foo"]
        assert any(isinstance(b.terminator, CFGReturn) for b in proc_cfg.blocks.values())
        has_y = any(
            any(isinstance(s, IRAssignConst) and s.name == "y" for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert not has_y

# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for liveness-based slot coalescing (Phase 5 core)."""

from __future__ import annotations

from compiler.cfg import build_cfg
from compiler.core_analyses import _liveness
from compiler.lowering import lower_to_ir
from compiler.slot_allocation import build_interference, coalesce_slots, slot_count
from compiler.ssa import build_ssa


def _setup(src: str, proc: str):
    m = lower_to_ir(src)
    cm = build_cfg(m)
    cfg = cm.procedures[proc]
    ssa = build_ssa(cfg)
    # Coalescing depends on liveness only (not the full analyse_function).
    _live_in, live_out = _liveness(cfg, ssa)
    return cfg, ssa, live_out


class TestInterference:
    def test_overlapping_locals_interfere(self):
        cfg, ssa, an = _setup("proc g {} { set a 1\n set b 2\n puts $a\n puts $b }", "::g")
        graph = build_interference(cfg, ssa, an)
        # a and b are both live between their defs and uses -> interfere.
        assert "b" in graph.get("a", set())
        assert "a" in graph.get("b", set())

    def test_disjoint_locals_do_not_interfere(self):
        cfg, ssa, an = _setup("proc f {} { set a 1\n puts $a\n set b 2\n puts $b }", "::f")
        graph = build_interference(cfg, ssa, an)
        assert "b" not in graph.get("a", set())


class TestCoalesce:
    def test_disjoint_ranges_share_one_slot(self):
        cfg, ssa, an = _setup(
            "proc f {} { set a 1\n puts $a\n set b 2\n puts $b\n set c 3\n puts $c }", "::f"
        )
        mapping = coalesce_slots(cfg, ssa, an)
        # Three sequentially-used locals collapse to a single reused slot.
        assert slot_count(mapping) == 1

    def test_overlapping_ranges_need_distinct_slots(self):
        cfg, ssa, an = _setup("proc g {} { set a 1\n set b 2\n puts $a\n puts $b }", "::g")
        mapping = coalesce_slots(cfg, ssa, an)
        assert mapping["a"] != mapping["b"]
        assert slot_count(mapping) == 2

    def test_params_get_stable_low_slots_and_never_share(self):
        # Parameters are live on entry, so they mutually interfere and keep
        # distinct low slots in incoming order.
        cfg, ssa, an = _setup("proc h {x y} { return [expr {$x + $y}] }", "::h")
        mapping = coalesce_slots(cfg, ssa, an, params=("x", "y"))
        assert mapping["x"] == 0
        assert mapping["y"] == 1

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

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import interp_instructions as tool


def test_total_instructions_reads_summary_or_totals() -> None:
    assert tool.total_instructions("events: Ir\nsummary: 12345\n") == 12345
    assert tool.total_instructions("events: Ir\ntotals: 678\n") == 678
    with pytest.raises(ValueError):
        tool.total_instructions("events: Ir\n")


def test_per_iteration_removes_fixed_cost() -> None:
    assert tool.per_iteration(1_000 + 300 * 50, 1_000 + 900 * 50) == 50


def test_render_requires_the_placeholder() -> None:
    assert tool.render("for {set i 0} {$i < @N@} {incr i} {}", 7).endswith(
        "< 7} {incr i} {}"
    )
    with pytest.raises(ValueError):
        tool.render("puts hi", 7)


def test_every_benchmark_has_the_placeholder() -> None:
    benchmarks = sorted(tool.BENCHMARKS.glob("*.tcl"))
    assert benchmarks
    for bench in benchmarks:
        tool.render(bench.read_text(), 1)


def test_engine_command_passes_quiet_to_the_runtime() -> None:
    script = Path("x.tcl")
    assert tool.engine_command("runtime", "/bin/rt", script) == [
        "/bin/rt",
        "--quiet",
        "x.tcl",
    ]
    assert tool.engine_command("tclvm", "/bin/vm", script) == ["/bin/vm", "x.tcl"]


def test_compare_flags_only_regressions_beyond_tolerance() -> None:
    base = {"tclvm": {"a": 100.0, "b": 100.0, "c": 100.0}, "runtime": {"a": 50.0}}
    head = {
        "tclvm": {"a": 101.0, "b": 110.0, "c": 80.0, "new": 5.0},
        "gone": {"a": 1.0},
    }
    rows, regressions = tool.compare(base, head, 0.02)
    assert [(r.engine, r.benchmark) for r in rows] == [
        ("tclvm", "a"),
        ("tclvm", "b"),
        ("tclvm", "c"),
    ]
    assert [(r.engine, r.benchmark) for r in regressions] == [("tclvm", "b")]


def test_compare_exit_status(tmp_path: Path) -> None:
    base = tmp_path / "base.json"
    head = tmp_path / "head.json"
    base.write_text(json.dumps({"tclvm": {"a": 100.0}}))
    head.write_text(json.dumps({"tclvm": {"a": 101.0}}))
    assert tool.main(["compare", str(base), str(head)]) == 0
    head.write_text(json.dumps({"tclvm": {"a": 103.0}}))
    assert tool.main(["compare", str(base), str(head)]) == 1

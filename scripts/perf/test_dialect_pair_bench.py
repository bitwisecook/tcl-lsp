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

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import dialect_pair_bench as bench


def test_corpus_files_selects_patterns_and_honours_excludes(tmp_path: Path) -> None:
    for rel in (
        "lib/a.tcl",
        "lib/b.txt",
        "auto.def",
        "examples/slow/x.tcl",
        ".git/c.tcl",
    ):
        path = tmp_path / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("set a 1\n")

    found = bench.corpus_files(
        tmp_path, bench.DEFAULT_PATTERNS, None, ("examples/slow/*",)
    )

    assert [p.relative_to(tmp_path).as_posix() for p in found] == [
        "auto.def",
        "lib/a.tcl",
    ]


def test_corpus_files_limit() -> None:
    root = Path(__file__).resolve().parent
    assert len(bench.corpus_files(root, ("*.py",), 1)) == 1


def test_summary_and_percentiles() -> None:
    values = [float(v) for v in range(1, 101)]
    out = bench.summary(values)
    assert out["n"] == 100
    assert out["p50_ms"] == 51.0
    assert out["p95_ms"] == 95.0
    assert out["max_ms"] == 100.0
    assert bench.summary([]) == {"n": 0}


def test_configuration_answer_pins_the_dialect() -> None:
    client = bench.DialectClient(".", ["true"], "jim")
    assert client.settings_for("tclLsp") == {"dialect": "jim"}
    assert client.settings_for(None) == {"tclLsp": {"dialect": "jim"}}
    assert client.settings_for("files") == {}

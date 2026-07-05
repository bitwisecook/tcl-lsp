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

"""Tests for conservative static loop evaluation helpers."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.static_loops import summarise_static_for_loop


def test_static_loop_handles_if_branching() -> None:
    env = summarise_static_for_loop(
        [
            "set i 0",
            "$i < 3",
            "incr i",
            "if {$i == 1} {set x 10} else {set x 20}",
        ]
    )
    assert env is not None
    assert env.get("i") == 3
    assert env.get("x") == 20


def test_static_loop_handles_switch_dispatch() -> None:
    env = summarise_static_for_loop(
        [
            "set i 0; set mode a",
            "$i < 1",
            "incr i",
            "switch $mode { a {set v 1} default {set v 9} }",
        ]
    )
    assert env is not None
    assert env.get("v") == 1


def test_static_loop_switch_requires_resolvable_subject() -> None:
    env = summarise_static_for_loop(
        [
            "set i 0",
            "$i < 1",
            "incr i",
            "switch $mode { a {set v 1} default {set v 9} }",
        ]
    )
    assert env is None

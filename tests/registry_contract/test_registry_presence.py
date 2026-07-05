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

"""Presence safety-net: the committed CSVs must match the live registry.

The CSVs under ``tests/baselines/registry/`` pin that every command (with
its arity and subcommand/switch counts), event, profile, and object is
present in the registry.  ``check_all`` regenerates the rows straight from
the registry and compares them to the committed CSVs, so this single test
guards both presence and staleness.
"""

from __future__ import annotations

from ._harness import BASELINE_DIR


def test_registry_fixtures_are_not_stale() -> None:
    from scripts.codegen.registry_baselines import check_all

    problems = check_all(BASELINE_DIR)
    assert not problems, "stale registry fixtures (run make gen-registry-baselines):\n" + "\n".join(
        problems
    )

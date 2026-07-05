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

"""Lightweight contract tests for refactoring-related Claude skills."""

from __future__ import annotations

from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]


def _read(path: str) -> str:
    return (_ROOT / path).read_text(encoding="utf-8")


def test_tcl_refactor_skill_references_refactor_subcommand():
    text = _read("ai/claude/skills/tcl-refactor/SKILL.md")
    assert "tcl_ai.py refactor" in text
    assert "Extract variable" in text
    assert "switch → dict" in text


def test_irule_datagroup_skill_references_new_datagroup_tools():
    text = _read("ai/claude/skills/irule-datagroup/SKILL.md")
    assert "tcl_ai.py suggest-datagroups" in text
    assert "tcl_ai.py extract-datagroup" in text

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

"""Tests for shared package suggestion ranking behaviour."""

from __future__ import annotations

from lsprotocol import types

import server.commands as server_module
import server.state as _lsp_state
from server.features.code_actions import get_code_actions
from server.features.package_suggestions import rank_package_suggestions


def _package_titles(actions: list[types.CodeAction]) -> list[str]:
    names: list[str] = []
    for action in actions:
        title = action.title
        prefix = "Add 'package require "
        if title.startswith(prefix) and title.endswith("'"):
            names.append(title[len(prefix) : -1])
    return names


def test_rank_package_suggestions_scoring_and_limit() -> None:
    packages = ["tls", "json", "json-tools", "xjsonx", "ajson", "sqlite"]

    assert rank_package_suggestions("json::parse", packages, 20) == [
        "json",
        "json-tools",
        "ajson",
        "xjsonx",
    ]
    assert rank_package_suggestions("json::parse", packages, 2) == ["json", "json-tools"]
    assert rank_package_suggestions("x", packages, 20) == []


def test_code_actions_and_server_command_use_identical_ranking(
    monkeypatch,
) -> None:
    packages = ["tls", "json", "json-tools", "xjsonx", "ajson", "sqlite"]
    symbol = "json::parse"

    monkeypatch.setattr(_lsp_state.package_resolver, "all_package_names", lambda: packages)
    server_suggestions = server_module.on_suggest_packages_for_symbol(symbol)["suggestions"]

    source = f"{symbol}\n"
    context = types.CodeActionContext(diagnostics=[])
    cursor = types.Range(
        start=types.Position(line=0, character=2),
        end=types.Position(line=0, character=2),
    )
    actions = get_code_actions(source, cursor, context, package_names=packages)
    action_suggestions = _package_titles(actions)

    assert action_suggestions == server_suggestions[:5]

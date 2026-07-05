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

"""Consumer-facing coverage for refactoring migration surfaces."""

from __future__ import annotations

import json

from ai.claude.tcl_ai import cmd_refactor
from ai.mcp.tcl_mcp_server import _handle_tools_list, _tool_refactor


def test_mcp_tool_registry_exposes_refactoring_suite():
    tools = _handle_tools_list({})["tools"]
    names = {tool["name"] for tool in tools}
    expected = {
        "refactor",
        "extract_variable",
        "inline_variable",
        "if_to_switch",
        "switch_to_dict",
        "brace_expr",
        "extract_datagroup",
        "suggest_datagroup_extractions",
    }
    assert expected.issubset(names)


def test_mcp_refactor_lists_if_to_switch_for_chain():
    source = 'if {$x eq "a"} {\n    puts alpha\n} elseif {$x eq "b"} {\n    puts beta\n}'
    payload = json.loads(_tool_refactor(source, 0, 0, 0, 0))
    tools = {item["tool"] for item in payload["available"]}
    assert "if_to_switch" in tools


def test_mcp_refactor_lists_extract_variable_when_selection_is_present():
    source = "puts [expr {$a + $b}]"
    payload = json.loads(_tool_refactor(source, 0, 5, 0, 20))
    tools = {item["tool"] for item in payload["available"]}
    assert "extract_variable" in tools


def test_claude_refactor_command_lists_new_refactorings(capsys):
    source = (
        "switch -exact -- $method {\n"
        "    GET { set handler handle_get }\n"
        "    POST { set handler handle_post }\n"
        "    PUT { set handler handle_put }\n"
        "}"
    )
    cmd_refactor(source, "/tmp/example.tcl")
    output = capsys.readouterr().out
    assert "`switch_to_dict`" in output

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

"""Consumer-facing coverage for minify/unminify MCP surfaces."""

from __future__ import annotations

import json

from ai.mcp.tcl_mcp_server import _handle_tools_list, _tool_unminify_error


def test_mcp_tool_registry_exposes_unminify_error() -> None:
    tools = _handle_tools_list({})["tools"]
    names = {tool["name"] for tool in tools}
    assert "unminify_error" in names


def test_mcp_unminify_error_translates_symbols() -> None:
    payload = json.loads(
        _tool_unminify_error(
            'can\'t read "a": no such variable',
            "# Variables in ::demo\n  a <- request_uri\n",
        )
    )
    assert payload["changed"] is True
    assert '"request_uri"' in payload["translated_error"]

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

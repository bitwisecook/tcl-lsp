"""Consumer-facing coverage for iRules registry metadata surfaces."""

from __future__ import annotations

import json
from pathlib import Path

from ai.claude.tcl_ai import cmd_command_info, cmd_event_info
from ai.mcp.tcl_mcp_server import _handle_tools_list, _tool_command_info, _tool_event_info
from lsp.commands import on_describe_irule_command

ROOT = Path(__file__).resolve().parents[1]


def _read(rel_path: str) -> str:
    return (ROOT / rel_path).read_text(encoding="utf-8")


def test_mcp_registry_uses_current_event_and_command_argument_names() -> None:
    tools = {tool["name"]: tool for tool in _handle_tools_list({})["tools"]}

    event_info = tools["event_info"]["inputSchema"]
    assert sorted(event_info["properties"]) == ["event_name"]
    assert event_info["required"] == ["event_name"]

    command_info = tools["command_info"]["inputSchema"]
    assert sorted(command_info["properties"]) == ["command_name"]
    assert command_info["required"] == ["command_name"]


def test_mcp_command_info_inherits_namespace_profile_events() -> None:
    payload = json.loads(_tool_command_info("ACCESS::log"))

    assert payload["found"] is True
    assert payload["command"] == "ACCESS::log"
    assert payload["valid_events"]
    assert "HTTP_REQUEST" not in payload["valid_events"]
    assert any(name.startswith("ACCESS_") for name in payload["valid_events"])


def test_mcp_event_info_exposes_transport_and_profiles() -> None:
    payload = json.loads(_tool_event_info("CLIENT_ACCEPTED"))

    assert payload["event"] == "CLIENT_ACCEPTED"
    assert payload["transport"] == "tcp/udp"
    assert payload["side"] == "client-side"


def test_lsp_command_description_inherits_namespace_profile_requirements() -> None:
    payload = on_describe_irule_command("ACCESS::log")

    assert payload["found"] is True
    assert payload["anyEvent"] is False
    assert payload["eventRequires"]["profiles"] == ["ACCESS"]
    assert any(name.startswith("ACCESS_") for name in payload["validEvents"])


def test_lsp_command_description_any_event_command() -> None:
    """RESOLV::lookup is ANY_EVENT per BIG-IP manpage."""
    payload = on_describe_irule_command("RESOLV::lookup")

    assert payload["found"] is True
    assert payload["anyEvent"] is True
    assert payload["eventRequires"]["profiles"] == []
    assert "RULE_INIT" in payload["validEvents"]
    assert "HTTP_REQUEST" in payload["validEvents"]


def test_claude_command_info_reports_namespace_profile_events(capsys) -> None:
    cmd_command_info("ACCESS::log")
    output = capsys.readouterr().out

    assert "Valid in:" in output
    assert "any event" not in output.lower()
    assert "ACCESS_" in output


def test_claude_event_info_reports_transport_and_profiles(capsys) -> None:
    cmd_event_info("CLIENTSSL_HANDSHAKE")
    output = capsys.readouterr().out

    assert "Transport: tcp" in output
    assert "Profiles:" in output
    assert "CLIENTSSL" in output


def test_zed_slash_commands_document_current_mcp_argument_names() -> None:
    text = _read("editors/zed/src/lib.rs")

    assert "command_name" in text
    assert "event_name" in text
    assert '`{{\\"name\\": "' not in text

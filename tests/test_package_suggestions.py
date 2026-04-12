"""Tests for shared package suggestion ranking behaviour."""

from __future__ import annotations

from lsprotocol import types

import lsp.server as server_module
from lsp.features.code_actions import get_code_actions
from lsp.features.package_suggestions import rank_package_suggestions


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

    monkeypatch.setattr(server_module.package_resolver, "all_package_names", lambda: packages)
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

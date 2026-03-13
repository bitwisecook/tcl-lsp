"""Tests for custom server workspace command helpers."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

import lsp.server as server_module
from core.bigip.parser import parse_bigip_conf


def test_describe_irule_event_reports_known_event() -> None:
    data = server_module.on_describe_irule_event("HTTP_REQUEST")
    assert data["event"] == "HTTP_REQUEST"
    assert data["known"] is True
    assert data["validCommandCount"] >= 1


def test_describe_irule_command_reports_metadata() -> None:
    data = server_module.on_describe_irule_command("HTTP::header")
    assert data["found"] is True
    assert data["command"] == "HTTP::header"
    assert isinstance(data["synopsis"], list)
    assert isinstance(data["switches"], list)


def test_apply_non_overlapping_fixes_skips_overlaps() -> None:
    rewritten, applied = server_module._apply_non_overlapping_fixes(
        "abcdef",
        [
            (0, 0, "X", "A", "fix A"),
            (0, 1, "YY", "B", "fix B"),
            (3, 3, "Q", "C", "fix C"),
        ],
    )
    # The overlap at (0,1) is skipped; non-overlapping edits are applied.
    assert rewritten == "XbcQef"
    assert [entry["code"] for entry in applied] == ["A", "C"]


def test_apply_safe_fixes_iteratively_applies_w100_then_w110() -> None:
    # W110 now only fires when a string literal operand is present (AST-based).
    # Line 1 triggers W100 (unbraced if), line 2 triggers W110 (string compare).
    source = 'if $a { puts yes }\nif {$x == "hello"} { puts y }\n'
    rewritten, applied = server_module._apply_safe_fixes_iteratively(source)
    assert "if {$a} { puts yes }" in rewritten
    assert any(entry["code"] == "W100" for entry in applied)
    assert any(entry["code"] == "W110" for entry in applied)


def test_suggest_packages_for_symbol_has_stable_shape() -> None:
    data = server_module.on_suggest_packages_for_symbol("json::parse")
    assert data["symbol"] == "json::parse"
    assert "suggestions" in data
    assert isinstance(data["suggestions"], list)


def test_extract_linked_objects_command_returns_graph(monkeypatch: pytest.MonkeyPatch) -> None:
    source = (
        "ltm pool /Common/web_pool { members { /Common/web1:80 { } } }\n"
        "ltm virtual /Common/www_vs { pool /Common/web_pool }\n"
    )
    uri = "file:///bigip.conf"
    cfg = parse_bigip_conf(source)

    class _DummyScanner:
        def __init__(self) -> None:
            self._cfgs = {uri: cfg}

        @property
        def bigip_configs(self) -> dict:
            return dict(self._cfgs)

        def parse_bigip_source(self, parse_uri: str, parse_source: str):
            parsed = parse_bigip_conf(parse_source)
            self._cfgs[parse_uri] = parsed
            return parsed

    monkeypatch.setattr(server_module, "background_scanner", _DummyScanner())
    monkeypatch.setattr(
        server_module,
        "_get_doc_source",
        lambda requested_uri: source if requested_uri == uri else "",
    )

    offset = source.index("ltm pool /Common/web_pool") + 10
    result = server_module.on_extract_linked_objects(uri, offset, 3, 100)
    assert result is not None
    headers = {node["header"] for node in result["nodes"]}
    assert "ltm pool /Common/web_pool" in headers
    assert "ltm virtual /Common/www_vs" in headers
    assert "roots" in result
    assert result["root"] == result["roots"][0]["id"]
    # Nodes carry sourceOrigin annotation.
    origins = {node["header"]: node["sourceOrigin"] for node in result["nodes"]}
    assert origins["ltm pool /Common/web_pool"] == "synced"
    assert origins["ltm virtual /Common/www_vs"] == "synced"


def test_extract_linked_objects_command_with_extra_offsets(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    source = (
        "ltm pool /Common/pool_a { members { /Common/10.0.0.1:80 { } } }\n"
        "ltm pool /Common/pool_b { members { /Common/10.0.0.2:80 { } } }\n"
        "ltm virtual /Common/vs_a { pool /Common/pool_a }\n"
        "ltm virtual /Common/vs_b { pool /Common/pool_b }\n"
    )
    uri = "file:///bigip.conf"
    cfg = parse_bigip_conf(source)

    class _DummyScanner:
        def __init__(self) -> None:
            self._cfgs = {uri: cfg}

        @property
        def bigip_configs(self) -> dict:
            return dict(self._cfgs)

        def parse_bigip_source(self, parse_uri: str, parse_source: str):
            parsed = parse_bigip_conf(parse_source)
            self._cfgs[parse_uri] = parsed
            return parsed

    monkeypatch.setattr(server_module, "background_scanner", _DummyScanner())
    monkeypatch.setattr(
        server_module,
        "_get_doc_source",
        lambda requested_uri: source if requested_uri == uri else "",
    )

    offset_a = source.index("ltm virtual /Common/vs_a") + 5
    offset_b = source.index("ltm virtual /Common/vs_b") + 5
    result = server_module.on_extract_linked_objects(
        uri,
        offset_a,
        3,
        200,
        extra_offsets=[[uri, offset_b]],
    )
    assert result is not None
    assert len(result["roots"]) == 2
    headers = {node["header"] for node in result["nodes"]}
    assert "ltm virtual /Common/vs_a" in headers
    assert "ltm virtual /Common/vs_b" in headers
    assert "ltm pool /Common/pool_a" in headers
    assert "ltm pool /Common/pool_b" in headers

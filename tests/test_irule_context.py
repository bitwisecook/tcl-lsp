"""Tests for ``ai.shared.irule_context`` and the MCP wrapper."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ai.shared.irule_context import (
    build_irule_context,
    context_bundle_to_dict,
    context_bundle_to_text,
)
from core.bigip.parser import parse_bigip_conf

_SAMPLE_CONF = """\
ltm pool /Common/web_pool {
    members { /Common/n1:80 { address 10.0.0.1 } }
    monitor /Common/http
}
ltm node /Common/n1 { address 10.0.0.1 }
ltm monitor http /Common/http { }
ltm data-group internal /Common/dg { type string records { foo { } } }
ltm rule /Common/foo {
when HTTP_REQUEST {
    if { [class match [HTTP::host] equals /Common/dg] } {
        pool /Common/web_pool
    } else {
        pool /Common/missing
    }
}
}
"""


def _bundle():
    cfg = parse_bigip_conf(_SAMPLE_CONF)
    rule = cfg.rules["/Common/foo"]
    return (
        cfg,
        rule,
        build_irule_context(
            rule,
            cfg,
            sources={"inline://0": _SAMPLE_CONF},
            config_origin="inline://0",
        ),
    )


def test_build_irule_context_resolves_pool_and_data_group():
    _cfg, _rule, bundle = _bundle()
    assert "/Common/web_pool" in bundle.pools
    assert "/Common/dg" in bundle.data_groups


def test_build_irule_context_records_unresolved():
    _cfg, _rule, bundle = _bundle()
    assert "pool" in bundle.unresolved
    assert "/Common/missing" in bundle.unresolved["pool"]


def test_build_irule_context_transitive_pulls_node_and_monitor():
    _cfg, _rule, bundle = _bundle()
    # Pool member /Common/n1:80 → node /Common/n1
    assert "/Common/n1" in bundle.nodes
    # Pool monitor /Common/http → monitor /Common/http
    assert "/Common/http" in bundle.monitors


def test_build_irule_context_no_transitive_skips_pool_chase():
    cfg = parse_bigip_conf(_SAMPLE_CONF)
    rule = cfg.rules["/Common/foo"]
    bundle = build_irule_context(rule, cfg, transitive=False)
    assert "/Common/web_pool" in bundle.pools
    assert bundle.nodes == {}


def test_context_bundle_to_dict_shape():
    _cfg, _rule, bundle = _bundle()
    payload = context_bundle_to_dict(bundle)
    assert payload["rule"]["fullPath"] == "/Common/foo"
    pool_paths = {p["fullPath"] for p in payload["pools"]}
    assert pool_paths == {"/Common/web_pool"}
    assert payload["unresolved"]["pool"] == ["/Common/missing"]


def test_context_bundle_to_text_uses_source_slices():
    _cfg, _rule, bundle = _bundle()
    text = context_bundle_to_text(bundle)
    # Rule body and referenced objects are present.
    assert "iRule /Common/foo" in text
    assert "referenced pool /Common/web_pool" in text
    assert "referenced data-group /Common/dg" in text
    # Source slices preserve the original Tcl text verbatim.
    assert "ltm pool /Common/web_pool" in text
    # Unresolved pool surfaces.
    assert "pool: /Common/missing" in text


def test_context_bundle_to_text_synthetic_when_no_sources():
    cfg = parse_bigip_conf(_SAMPLE_CONF)
    rule = cfg.rules["/Common/foo"]
    bundle = build_irule_context(rule, cfg)  # no sources
    text = context_bundle_to_text(bundle)
    # Rule body is still rendered (synthetic stanza wrapper).
    assert "iRule /Common/foo" in text
    assert "ltm rule /Common/foo {" in text


# ---------------------------------------------------------------------------
# MCP tool
# ---------------------------------------------------------------------------


def test_mcp_irule_with_context_json():
    from ai.mcp.tcl_mcp_server import _tool_irule_with_context

    payload = json.loads(
        _tool_irule_with_context(
            rule_path="/Common/foo",
            config_text=_SAMPLE_CONF,
            format="json",
        )
    )
    assert payload["format"] == "json"
    assert len(payload["bundles"]) == 1
    bundle = payload["bundles"][0]
    assert bundle["rule"]["fullPath"] == "/Common/foo"
    assert any(p["fullPath"] == "/Common/web_pool" for p in bundle["pools"])


def test_mcp_irule_with_context_text():
    from ai.mcp.tcl_mcp_server import _tool_irule_with_context

    payload = json.loads(
        _tool_irule_with_context(
            rule_path="/Common/foo",
            config_text=_SAMPLE_CONF,
            format="text",
        )
    )
    assert payload["format"] == "text"
    text = payload["bundles"][0]["text"]
    assert "iRule /Common/foo" in text
    assert "referenced pool /Common/web_pool" in text


def test_mcp_irule_with_context_all_rules_when_path_blank():
    from ai.mcp.tcl_mcp_server import _tool_irule_with_context

    payload = json.loads(_tool_irule_with_context(config_text=_SAMPLE_CONF))
    assert len(payload["bundles"]) == 1  # only one rule in sample


def test_mcp_irule_with_context_missing_input_returns_error():
    from ai.mcp.tcl_mcp_server import _tool_irule_with_context

    payload = json.loads(_tool_irule_with_context(rule_path="/Common/foo"))
    assert "error" in payload


def test_mcp_irule_with_context_unknown_rule_returns_error():
    from ai.mcp.tcl_mcp_server import _tool_irule_with_context

    payload = json.loads(
        _tool_irule_with_context(
            rule_path="/Common/nope",
            config_text=_SAMPLE_CONF,
        )
    )
    assert "error" in payload


def test_mcp_irule_with_context_ucs_path(tmp_path):
    """``config_paths`` must extract UCS archives, not feed gzip bytes
    straight to the parser (regression for the original Codex review)."""
    from ai.mcp.tcl_mcp_server import _tool_irule_with_context
    from explorer.f5_remote.ucs import make_test_ucs

    ucs_path = tmp_path / "device.ucs"
    ucs_path.write_bytes(
        make_test_ucs(
            {
                "config/bigip.conf": (
                    "ltm pool /Common/web_pool { members { /Common/n1:80 { } } }\n"
                    "ltm rule /Common/foo { when HTTP_REQUEST { pool /Common/web_pool } }\n"
                ),
            }
        )
    )
    payload = json.loads(
        _tool_irule_with_context(
            rule_path="/Common/foo",
            config_paths=[str(ucs_path)],
            format="json",
        )
    )
    assert payload["format"] == "json"
    bundle = payload["bundles"][0]
    assert bundle["rule"]["fullPath"] == "/Common/foo"
    pool_paths = {p["fullPath"] for p in bundle["pools"]}
    assert "/Common/web_pool" in pool_paths


def test_f5_irule_context_multi_file_keeps_source_slices(tmp_path, capsys):
    """Regression for the Codex P2 source-slice key-mismatch bug.

    With more than one file input the source-slice map and the config
    map must use the same keys so :func:`build_irule_context` can hand
    each rule's slice back to the renderer.
    """
    a = tmp_path / "a.conf"
    b = tmp_path / "b.conf"
    a.write_text(
        "ltm pool /Common/web_pool { members { /Common/n1:80 { } } }\n",
        encoding="utf-8",
    )
    b.write_text(
        "ltm rule /Common/foo { when HTTP_REQUEST { pool /Common/web_pool } }\n",
        encoding="utf-8",
    )

    from explorer.f5_cli import main

    rc = main(["irule", "context", str(a), str(b), "--rule", "/Common/foo"])
    out = capsys.readouterr().out
    # Slice from b.conf is reused verbatim — exact-text match,
    # not the synthetic ``ltm rule … {`` wrapper that build_irule_context
    # would emit when no source slice is available.
    assert rc == 0
    assert "ltm rule /Common/foo { when HTTP_REQUEST { pool /Common/web_pool } }" in out

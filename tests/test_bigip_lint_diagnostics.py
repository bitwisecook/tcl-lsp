"""Regression tests for the BIG-IP lint→LSP diagnostics bridge."""

from __future__ import annotations

from dialects.f5.bigip.diagnostics import get_bigip_lint_diagnostics
from dialects.f5.bigip.parser import parse_bigip_conf


def test_lint_surfaces_irule_missing_pool_diagnostic():
    """An iRule referencing a non-existent pool produces an
    ``irule-missing-pool`` diagnostic anchored on the rule's stanza
    range (which is what the editor needs to draw the squiggle).
    """
    src = (
        "ltm pool /Common/web_pool { }\n"
        "ltm virtual /Common/vs1 { destination /Common/1.1.1.1:80 pool /Common/web_pool }\n"
        "ltm rule /Common/r {\n"
        "when HTTP_REQUEST { pool /Common/missing_pool }\n"
        "}\n"
    )
    config = parse_bigip_conf(src)
    diags = get_bigip_lint_diagnostics(uri="file:///t.conf", source=src, config=config)
    codes = sorted({d.code for d in diags})
    assert "irule-missing-pool" in codes
    rule_diag = next(d for d in diags if d.code == "irule-missing-pool")
    assert "/Common/missing_pool" in rule_diag.message


def test_lint_diagnostics_respect_disabled_codes():
    """``disabled_codes`` filters lint findings before they reach the editor."""
    src = (
        "ltm pool /Common/p { }\n"
        "ltm virtual /Common/vs1 { destination /Common/1.1.1.1:80 pool /Common/p }\n"
        "ltm rule /Common/r {\n"
        "when HTTP_REQUEST { pool /Common/missing_pool }\n"
        "}\n"
    )
    config = parse_bigip_conf(src)
    diags = get_bigip_lint_diagnostics(
        uri="file:///t.conf",
        source=src,
        config=config,
        disabled_codes={"irule-missing-pool"},
    )
    assert all(d.code != "irule-missing-pool" for d in diags)


def test_lint_diagnostics_walk_workspace_configs():
    """Cross-file references resolve through ``workspace_configs``: a
    pool defined in fileB satisfies an iRule reference in fileA, so
    no ``irule-missing-pool`` finding is emitted for the cross-file
    case.
    """
    file_a = (
        "ltm virtual /Common/vs1 { destination /Common/1.1.1.1:80 pool /Common/p }\n"
        "ltm rule /Common/r {\n"
        "when HTTP_REQUEST { pool /Common/p }\n"
        "}\n"
    )
    file_b = "ltm pool /Common/p { }\n"
    config_a = parse_bigip_conf(file_a)
    config_b = parse_bigip_conf(file_b)
    diags = get_bigip_lint_diagnostics(
        uri="file:///a.conf",
        source=file_a,
        config=config_a,
        workspace_sources={"file:///b.conf": file_b},
        workspace_configs={"file:///b.conf": config_b},
    )
    assert all(d.code != "irule-missing-pool" for d in diags)

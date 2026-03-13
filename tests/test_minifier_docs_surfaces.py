"""Documentation surface checks for minify/unminify features."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _read(rel_path: str) -> str:
    return (ROOT / rel_path).read_text(encoding="utf-8")


def test_kcs_feature_index_lists_minify_and_unminify() -> None:
    index = _read("docs/kcs/features/README.md")
    assert "kcs-feature-minifier.md" in index
    assert "kcs-feature-unminify-error.md" in index


def test_mcp_kcs_lists_unminify_error_tool() -> None:
    kcs = _read("docs/kcs/features/kcs-feature-mcp-server.md")
    assert "`unminify_error`" in kcs

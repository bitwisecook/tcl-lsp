"""Tests for the KCS help database build and runtime query API."""

from __future__ import annotations

import sqlite3
import tempfile
from collections.abc import Generator
from pathlib import Path

import pytest

# Build script tests


@pytest.fixture(scope="module")
def sample_features_dir(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """Create a temporary directory with sample KCS feature markdown files."""
    d = tmp_path_factory.mktemp("kcs_features")

    (d / "kcs-feature-hover.md").write_text(
        """\
# KCS: feature — Hover

## Summary

Command documentation and proc signatures on hover.

## Surface

lsp, mcp, all-editors

## How to use

- **Editor**: Hover over any symbol.
- **MCP**: `hover` tool.

## Operational context

The hover provider resolves the symbol under the cursor.

## File-path anchors

- `lsp/features/hover.py`

## Failure modes

- Missing hover after updates.

## Test anchors

- `tests/test_hover.py`

## Screenshots

- `02-hover-proc` — hover showing proc signature

## Discoverability

- [KCS feature index](README.md)
""",
        encoding="utf-8",
    )

    (d / "kcs-feature-completions.md").write_text(
        """\
# KCS: feature — Completions

## Summary

Command, variable, and switch completions.

## Surface

lsp, all-editors

## How to use

- **Editor**: Type and press Ctrl+Space.

## Screenshots

- `03-completions` — completion list
""",
        encoding="utf-8",
    )

    # A file without a valid title (should be skipped)
    (d / "kcs-feature-invalid.md").write_text(
        "# Not a KCS feature\n\nJust some text.\n",
        encoding="utf-8",
    )

    return d


@pytest.fixture(scope="module")
def sample_screenshots_dir(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """Create a temporary directory with a fake screenshot PNG."""
    d = tmp_path_factory.mktemp("screenshots")
    # Minimal valid PNG: 1x1 red pixel
    png_data = (
        b"\x89PNG\r\n\x1a\n"
        b"\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01"
        b"\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx"
        b"\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\x18\xd8N\x00"
        b"\x00\x00\x00IEND\xaeB`\x82"
    )
    (d / "02-hover-proc.png").write_bytes(png_data)
    return d


@pytest.fixture(scope="module")
def built_db(
    sample_features_dir: Path,
    sample_screenshots_dir: Path,
    tmp_path_factory: pytest.TempPathFactory,
) -> Path:
    """Build a KCS database from sample data and return its path."""
    # Temporarily override the module-level paths
    import scripts.build_kcs_db as mod
    from scripts.build_kcs_db import (
        build_database,
    )

    orig_features = mod._FEATURES_DIR
    orig_screenshots = mod._SCREENSHOTS_DIR
    try:
        mod._FEATURES_DIR = sample_features_dir
        mod._SCREENSHOTS_DIR = sample_screenshots_dir

        out = tmp_path_factory.mktemp("db") / "test_kcs.db"
        build_database(out, verbose=True)
        return out
    finally:
        mod._FEATURES_DIR = orig_features
        mod._SCREENSHOTS_DIR = orig_screenshots


def test_build_creates_database(built_db: Path) -> None:
    assert built_db.exists()
    assert built_db.stat().st_size > 0


def test_build_contains_features(built_db: Path) -> None:
    conn = sqlite3.connect(str(built_db))
    rows = conn.execute("SELECT name, summary, surface, category FROM kcs_features").fetchall()
    conn.close()

    names = {r[0] for r in rows}
    assert "Hover" in names
    assert "Completions" in names
    assert len(names) == 2  # invalid.md should be skipped


def test_build_contains_screenshot(built_db: Path) -> None:
    conn = sqlite3.connect(str(built_db))
    rows = conn.execute(
        "SELECT ref_id, caption, mime_type, length(data) FROM screenshots"
    ).fetchall()
    conn.close()

    refs = {r[0]: r for r in rows}
    assert "02-hover-proc" in refs
    assert refs["02-hover-proc"][2] == "image/png"
    assert refs["02-hover-proc"][3] > 0

    # 03-completions should exist but with empty data (file not present)
    assert "03-completions" in refs
    assert refs["03-completions"][3] == 0


def test_build_fts_search(built_db: Path) -> None:
    conn = sqlite3.connect(str(built_db))
    rows = conn.execute("SELECT name FROM kcs_features WHERE kcs_features MATCH 'hover'").fetchall()
    conn.close()

    assert any(r[0] == "Hover" for r in rows)


def test_build_surface_category(built_db: Path) -> None:
    conn = sqlite3.connect(str(built_db))
    rows = conn.execute("SELECT name, category FROM kcs_features").fetchall()
    conn.close()

    cats = {r[0]: r[1] for r in rows}
    assert cats["Hover"] == "LSP + AI Features"  # has mcp
    assert cats["Completions"] == "LSP Features (all editors)"  # no mcp


# Runtime query API tests


@pytest.fixture()
def kcs_db_connection(
    built_db: Path, monkeypatch: pytest.MonkeyPatch
) -> Generator[None, None, None]:
    """Set up kcs_db module to use the test database."""
    import core.help.kcs_db as kcs_mod

    # Reset any previous connection
    kcs_mod.close()

    # Monkeypatch _load_db_bytes to return our test database
    db_bytes = built_db.read_bytes()
    monkeypatch.setattr(kcs_mod, "_load_db_bytes", lambda: db_bytes)

    yield

    kcs_mod.close()


def test_search_help(kcs_db_connection: None) -> None:
    from core.help.kcs_db import search_help

    results = search_help("hover")
    assert len(results) >= 1
    assert results[0]["name"] == "Hover"


def test_search_help_no_results(kcs_db_connection: None) -> None:
    from core.help.kcs_db import search_help

    results = search_help("nonexistent_xyzzy")
    assert len(results) == 0


def test_get_feature(kcs_db_connection: None) -> None:
    from core.help.kcs_db import get_feature

    feat = get_feature("Hover")
    assert feat is not None
    assert feat["name"] == "Hover"
    assert "screenshots" in feat
    assert feat["screenshots"][0]["ref_id"] == "02-hover-proc"
    assert feat["screenshots"][0]["has_image"] is True


def test_get_feature_case_insensitive(kcs_db_connection: None) -> None:
    from core.help.kcs_db import get_feature

    feat = get_feature("hover")
    assert feat is not None
    assert feat["name"] == "Hover"


def test_get_feature_missing(kcs_db_connection: None) -> None:
    from core.help.kcs_db import get_feature

    assert get_feature("NoSuchFeature") is None


def test_get_screenshot(kcs_db_connection: None) -> None:
    from core.help.kcs_db import get_screenshot

    result = get_screenshot("02-hover-proc")
    assert result is not None
    data, mime = result
    assert mime == "image/png"
    assert len(data) > 0
    assert data[:4] == b"\x89PNG"


def test_get_screenshot_missing(kcs_db_connection: None) -> None:
    from core.help.kcs_db import get_screenshot

    assert get_screenshot("nonexistent") is None


def test_get_screenshot_base64(kcs_db_connection: None) -> None:
    import base64

    from core.help.kcs_db import get_screenshot_base64

    result = get_screenshot_base64("02-hover-proc")
    assert result is not None
    assert result["mime_type"] == "image/png"
    decoded = base64.b64decode(result["data"])
    assert decoded[:4] == b"\x89PNG"


def test_list_features(kcs_db_connection: None) -> None:
    from core.help.kcs_db import list_features

    catalogue = list_features()
    assert isinstance(catalogue, dict)
    assert len(catalogue) >= 1

    # Check Hover is in the right category
    lsp_ai = catalogue.get("LSP + AI Features", [])
    names = [f["name"] for f in lsp_ai]
    assert "Hover" in names


def test_list_screenshots_for_feature(kcs_db_connection: None) -> None:
    from core.help.kcs_db import list_screenshots_for_feature

    screenshots = list_screenshots_for_feature("kcs-feature-hover.md")
    assert len(screenshots) == 1
    assert screenshots[0]["ref_id"] == "02-hover-proc"
    assert screenshots[0]["has_image"] is True


# Loading path tests


def test_deserialize_path(built_db: Path) -> None:
    """Test the deserialize loading path (Python 3.12+)."""
    import core.help.kcs_db as kcs_mod

    if not kcs_mod._has_deserialize():
        pytest.skip("deserialize not available (Python < 3.12)")

    db_bytes = built_db.read_bytes()
    conn = sqlite3.connect(":memory:")
    conn.deserialize(db_bytes)  # type: ignore[attr-defined]  # Python 3.12+
    conn.row_factory = sqlite3.Row

    rows = conn.execute("SELECT name FROM kcs_features").fetchall()
    assert len(rows) == 2
    conn.close()


def test_tempfile_fallback_path(built_db: Path) -> None:
    """Test the tempfile loading path (simulates Python < 3.12)."""
    import os

    db_bytes = built_db.read_bytes()

    fd, path = tempfile.mkstemp(suffix=".db", prefix="kcs_help_test_")
    try:
        os.write(fd, db_bytes)
        os.close(fd)
        conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
        conn.row_factory = sqlite3.Row
        rows = conn.execute("SELECT name FROM kcs_features").fetchall()
        assert len(rows) == 2
        conn.close()
    finally:
        os.unlink(path)

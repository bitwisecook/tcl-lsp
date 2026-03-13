"""Runtime loader and query API for the KCS help database.

Loading strategy:
- Python 3.12+: ``sqlite3.Connection.deserialize()`` from in-memory bytes
- Python 3.10–3.11: extract to a temporary file, open read-only

All queries go through a lazily-initialised singleton connection.
"""

from __future__ import annotations

import atexit
import base64
import os
import sqlite3
import tempfile
from typing import Any

_conn: sqlite3.Connection | None = None
_tmpfile: str | None = None


def _has_deserialize() -> bool:
    """Check if sqlite3.Connection has deserialize() (Python 3.12+)."""
    return hasattr(sqlite3.Connection, "deserialize")


def _load_db_bytes() -> bytes:
    """Read the bundled kcs_help.db via importlib.resources."""
    from importlib.resources import files

    return files("core.help").joinpath("kcs_help.db").read_bytes()


def load_help_db() -> sqlite3.Connection:
    """Return a read-only connection to the KCS help database.

    Uses ``deserialize()`` on Python 3.12+ for zero-copy in-memory loading.
    Falls back to a temporary file on older Pythons.
    """
    global _conn, _tmpfile  # noqa: PLW0603

    if _conn is not None:
        return _conn

    db_bytes = _load_db_bytes()

    if _has_deserialize():
        _conn = sqlite3.connect(":memory:")
        _conn.deserialize(db_bytes)  # type: ignore[attr-defined]  # Python 3.12+
    else:
        # Fallback: write to a temp file and open read-only
        fd, path = tempfile.mkstemp(suffix=".db", prefix="kcs_help_")
        try:
            os.write(fd, db_bytes)
        finally:
            os.close(fd)
        _tmpfile = path
        _conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
        atexit.register(_cleanup_tmpfile)

    _conn.row_factory = sqlite3.Row
    return _conn


def _cleanup_tmpfile() -> None:
    """Remove the temporary database file on interpreter exit."""
    global _tmpfile  # noqa: PLW0603
    if _tmpfile and os.path.exists(_tmpfile):
        try:
            os.unlink(_tmpfile)
        except OSError:
            pass
        _tmpfile = None


def close() -> None:
    """Close the database connection and clean up."""
    global _conn  # noqa: PLW0603
    if _conn is not None:
        _conn.close()
        _conn = None
    _cleanup_tmpfile()


# Query helpers


def search_help(query: str, *, limit: int = 20) -> list[dict[str, Any]]:
    """Full-text search across all KCS features, ranked by relevance."""
    conn = load_help_db()
    # Escape special FTS5 characters and append * for prefix matching
    safe_query = query.replace('"', '""')
    try:
        rows = conn.execute(
            """
            SELECT name, summary, surface, category, how_to_use, file,
                   rank
            FROM kcs_features
            WHERE kcs_features MATCH ?
            ORDER BY rank
            LIMIT ?
            """,
            (f'"{safe_query}" OR {safe_query}*', limit),
        ).fetchall()
    except sqlite3.OperationalError:
        # Fallback: simple LIKE search if FTS query syntax fails
        rows = conn.execute(
            """
            SELECT name, summary, surface, category, how_to_use, file,
                   0 as rank
            FROM kcs_features
            WHERE name LIKE ? OR summary LIKE ? OR how_to_use LIKE ?
            ORDER BY name
            LIMIT ?
            """,
            (f"%{query}%", f"%{query}%", f"%{query}%", limit),
        ).fetchall()

    return [dict(row) for row in rows]


def get_feature(name: str) -> dict[str, Any] | None:
    """Look up a feature by exact name (case-insensitive)."""
    conn = load_help_db()
    row = conn.execute(
        """
        SELECT name, summary, surface, category, how_to_use, content, file
        FROM kcs_features
        WHERE name = ? COLLATE NOCASE
        """,
        (name,),
    ).fetchone()
    if row is None:
        return None
    result = dict(row)
    # Attach screenshot metadata
    screenshots = conn.execute(
        "SELECT ref_id, caption, mime_type FROM screenshots WHERE kcs_file = ?",
        (result["file"],),
    ).fetchall()
    if screenshots:
        result["screenshots"] = [
            {"ref_id": r["ref_id"], "caption": r["caption"], "has_image": bool(r["mime_type"])}
            for r in screenshots
        ]
    return result


def get_screenshot(ref_id: str) -> tuple[bytes, str] | None:
    """Return (image_data, mime_type) for a screenshot reference ID."""
    conn = load_help_db()
    row = conn.execute(
        "SELECT data, mime_type FROM screenshots WHERE ref_id = ? AND mime_type != ''",
        (ref_id,),
    ).fetchone()
    if row is None:
        return None
    return bytes(row["data"]), row["mime_type"]


def get_screenshot_base64(ref_id: str) -> dict[str, str] | None:
    """Return screenshot as base64-encoded dict with mime type."""
    result = get_screenshot(ref_id)
    if result is None:
        return None
    data, mime = result
    return {"data": base64.b64encode(data).decode("ascii"), "mime_type": mime}


def list_features() -> dict[str, list[dict[str, Any]]]:
    """Return all features grouped by category (replaces _build_feature_catalogue)."""
    conn = load_help_db()
    rows = conn.execute(
        """
        SELECT name, summary, surface, category, how_to_use, file
        FROM kcs_features
        ORDER BY category, name
        """
    ).fetchall()

    catalogue: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        d = dict(row)
        cat = d.pop("category")
        catalogue.setdefault(cat, []).append(d)

    return catalogue


def list_screenshots_for_feature(kcs_file: str) -> list[dict[str, Any]]:
    """Return screenshot metadata for a given feature file."""
    conn = load_help_db()
    rows = conn.execute(
        "SELECT ref_id, caption, mime_type FROM screenshots WHERE kcs_file = ?",
        (kcs_file,),
    ).fetchall()
    return [
        {"ref_id": r["ref_id"], "caption": r["caption"], "has_image": bool(r["mime_type"])}
        for r in rows
    ]

"""Runtime loader and query API for the KCS help database.

Loading strategy:
- Python 3.12+: ``sqlite3.Connection.deserialize()`` from in-memory bytes
- Python 3.10–3.11: extract to a temporary file, open read-only

All queries go through a lazily-initialised singleton connection.

This module also defines the canonical editor/tool vocabulary used in KCS
``## Applies to`` lines and the helpers (:func:`parse_applies_to`,
:func:`applies_to_category`) that parse them. The build script and the
feature-catalogue generator both import these so the vocabulary stays in
one place.
"""

from __future__ import annotations

import atexit
import base64
import os
import sqlite3
import tempfile
from typing import Any

# ``## Applies to`` tags. A KCS note's Applies to line is a comma-separated
# plain-English list of editors and tools. The build script parses it into
# a set of normalised tags (lowercased, with spaces replaced by hyphens)
# and stores one row per tag in the ``feature_tags`` table. Runtime
# queries join back through that table to answer "which features work in
# Zed?" style questions.
#
# This is the single source of truth for the vocabulary contributors write
# in KCS notes. Any rename must also update docs/kcs/STYLE.md (rule 11)
# and the templates under docs/kcs/templates/.

# Editors that are driven by the LSP server.
LSP_EDITOR_TAGS: frozenset[str] = frozenset(
    {
        "vs-code",
        "zed",
        "jetbrains",
        "neovim",
        "helix",
        "emacs",
        "sublime-text",
    }
)

# The plain-English display name for every known tag. Unknown tags fall
# back to title case with hyphens turned back into spaces. This is the
# single source of truth for the vocabulary; if you add a tag here you
# must also document it in docs/kcs/STYLE.md (rule 11).
TAG_DISPLAY: dict[str, str] = {
    # Editor tags (the LSP editor set is frozen in LSP_EDITOR_TAGS).
    "vs-code": "VS Code",
    "zed": "Zed",
    "jetbrains": "JetBrains",
    "neovim": "Neovim",
    "helix": "Helix",
    "emacs": "Emacs",
    "sublime-text": "Sublime Text",
    # Tool tags.
    "tcl-lsp-cli": "tcl-lsp CLI",
    "mcp": "MCP",
    "claude-skill": "Claude skill",
    "copilot-chat": "VS Code Copilot Chat",
    # KCS type tags (added automatically from the filename prefix).
    "issue": "Issue",
    "qa": "Q&A",
    "howto": "How-to",
    "feature": "Feature",
    "diagnostic": "Diagnostic",
    "optimisation": "Optimisation",
    # Content tags (what kind of thing the note documents).
    # `diagnostic` and `optimisation` are both type tags (auto-derived
    # from the filename) and content tags (hand-written in the Applies
    # to line of Functionality notes). The same key is used for both;
    # there is no conflict because they mean the same thing in both
    # roles.
    "warning": "Warning",
    "refactoring": "Refactoring",
    "analyser": "Analyser",
    "transform": "Transform",
    # Compiler pass tags. Attach to a code page to say which pass
    # produces the code, and to a feature page to say which passes
    # the feature consumes. See docs/GLOSSARY.md for definitions and
    # docs/design/compiler/ for the design docs each tag points at.
    "lexing": "Lexing",
    "lowering": "IR lowering",
    "cfg": "CFG construction",
    "ssa": "SSA construction",
    "sccp": "SCCP",
    "liveness": "Liveness",
    "type-infer": "Type inference",
    "gvn": "GVN",
    "cse": "CSE",
    "dce": "DCE",
    "licm": "LICM",
    "instcombine": "InstCombine",
    "ipa": "IPA",
    "memssa": "Memory-SSA",
    "dataflow": "Data-flow",
    "taint": "Taint",
    "shimmer": "Shimmer",
    "tail-call": "Tail-call",
    "code-sinking": "Code sinking",
    "unused-procs": "Unused procs",
    "side-effects": "Side-effects",
    "exec-intent": "Execution intent",
    "rendered-props": "Rendered properties",
    "const-fold": "Constant folding",
    "strength-reduce": "Strength reduction",
    "codegen": "Codegen",
}


def _normalise_tag(raw: str) -> str:
    """Normalise a single raw Applies to token to its canonical tag form.

    Lowercases the token, strips surrounding whitespace, and collapses
    internal whitespace runs into a single hyphen. Existing hyphens are
    preserved, so ``tcl-lsp CLI`` becomes ``tcl-lsp-cli``.
    """
    return "-".join(raw.strip().lower().split())


def parse_applies_to(text: str) -> set[str]:
    """Parse a KCS ``## Applies to`` line into a set of normalised tags.

    Input is the free-text body of the section — a comma-separated list of
    plain-English editor and tool names. Each token is lowercased, stripped,
    and has internal whitespace replaced with hyphens (so ``VS Code`` and
    ``vs code`` both normalise to ``vs-code``). The shorthand
    ``all editors`` expands to the full set of LSP editor tags, so
    downstream callers can treat the result uniformly and ``all-editors``
    is never stored in the tags table.
    """
    parts = {_normalise_tag(p) for p in text.split(",") if p.strip()}
    if "all-editors" in parts:
        parts.discard("all-editors")
        parts |= LSP_EDITOR_TAGS
    return parts


def display_tag(tag: str) -> str:
    """Return the human-readable form of a normalised tag."""
    return TAG_DISPLAY.get(tag, tag.replace("-", " ").title())


def applies_to_category(tags: set[str] | str) -> str:
    """Map a set of Applies to tags to a grouping category name.

    Accepts either an already-parsed tag set or the raw Applies to text;
    the raw text is passed through :func:`parse_applies_to` first.
    """
    if isinstance(tags, str):
        tags = parse_applies_to(tags)
    lsp_editor_count = len(tags & LSP_EDITOR_TAGS)
    has_lsp = lsp_editor_count >= 2
    has_copilot_chat = "copilot-chat" in tags
    has_claude_skill = "claude-skill" in tags
    has_mcp = "mcp" in tags
    has_vscode_only = "vs-code" in tags and lsp_editor_count < 2

    if has_lsp:
        if has_mcp or has_claude_skill:
            return "LSP + AI Features"
        return "LSP Features (all editors)"
    if has_copilot_chat:
        return "VS Code AI Chat"
    if has_claude_skill:
        return "Claude Code Skills"
    if has_mcp:
        return "MCP Tools"
    if has_vscode_only:
        return "VS Code Commands"
    return "Other"


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
        _conn.deserialize(db_bytes)  # type: ignore[attr-defined, unresolved-attribute]  # Python 3.12+
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


def _fetch_tags(conn: sqlite3.Connection, file: str) -> list[str]:
    """Return the sorted list of Applies to tags for a feature file."""
    rows = conn.execute(
        "SELECT tag FROM feature_tags WHERE file = ? ORDER BY tag",
        (file,),
    ).fetchall()
    return [r["tag"] for r in rows]


def _attach_tags(conn: sqlite3.Connection, result: dict[str, Any]) -> None:
    """Attach the ``applies_to`` tag list to a feature result dict in place."""
    result["applies_to"] = _fetch_tags(conn, result["file"])


def search_help(query: str, *, limit: int = 20) -> list[dict[str, Any]]:
    """Full-text search across all KCS features, ranked by relevance."""
    conn = load_help_db()
    # Escape special FTS5 characters and append * for prefix matching
    safe_query = query.replace('"', '""')
    try:
        rows = conn.execute(
            """
            SELECT name, summary, category, how_to_use, file, rank
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
            SELECT name, summary, category, how_to_use, file, 0 as rank
            FROM kcs_features
            WHERE name LIKE ? OR summary LIKE ? OR how_to_use LIKE ?
            ORDER BY name
            LIMIT ?
            """,
            (f"%{query}%", f"%{query}%", f"%{query}%", limit),
        ).fetchall()

    results = [dict(row) for row in rows]
    for r in results:
        _attach_tags(conn, r)
    return results


def get_feature(name: str) -> dict[str, Any] | None:
    """Look up a feature by exact name (case-insensitive)."""
    conn = load_help_db()
    row = conn.execute(
        """
        SELECT name, summary, category, how_to_use, content, file
        FROM kcs_features
        WHERE name = ? COLLATE NOCASE
        """,
        (name,),
    ).fetchone()
    if row is None:
        return None
    result = dict(row)
    _attach_tags(conn, result)
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


def features_with_tag(tag: str) -> list[dict[str, Any]]:
    """Return every feature whose Applies to line includes the given tag.

    ``tag`` is matched case-insensitively against the normalised tag
    vocabulary (``vs code``, ``zed``, ``mcp server``, …). Pass ``all
    editors`` and the helper expands to the full LSP editor set.
    """
    conn = load_help_db()
    normalised = parse_applies_to(tag)
    if not normalised:
        return []
    placeholders = ",".join("?" * len(normalised))
    rows = conn.execute(
        f"""
        SELECT DISTINCT f.name, f.summary, f.category, f.how_to_use, f.file
        FROM kcs_features f
        JOIN feature_tags t ON t.file = f.file
        WHERE t.tag IN ({placeholders})
        ORDER BY f.name
        """,
        tuple(normalised),
    ).fetchall()
    results = [dict(row) for row in rows]
    for r in results:
        _attach_tags(conn, r)
    return results


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
        SELECT name, summary, category, how_to_use, file
        FROM kcs_features
        ORDER BY category, name
        """
    ).fetchall()

    catalogue: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        d = dict(row)
        cat = d.pop("category")
        _attach_tags(conn, d)
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

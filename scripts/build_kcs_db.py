#!/usr/bin/env python3
"""Build the KCS help database from docs/kcs/features/ markdown files.

Parses every ``kcs-feature-*.md`` file, extracts structured sections, and
writes a SQLite database with FTS5 full-text search and screenshot BLOBs.

Output: ``core/help/kcs_help.db``

Usage::

    python scripts/build_kcs_db.py          # default paths
    python scripts/build_kcs_db.py --out /tmp/kcs_help.db  # custom output
"""

from __future__ import annotations

import argparse
import mimetypes
import re
import sqlite3
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parent.parent
_FEATURES_DIR = _REPO_ROOT / "docs" / "kcs" / "features"
_SCREENSHOTS_DIR = _REPO_ROOT / "docs" / "screenshots"
_DEFAULT_OUT = _REPO_ROOT / "core" / "help" / "kcs_help.db"

# Regex for KCS feature title line
_TITLE_RE = re.compile(r"^#\s+KCS:\s+feature\s+—\s+(.+)$", re.MULTILINE)
# Regex for screenshot references: - `ref-id` — caption text
_SCREENSHOT_RE = re.compile(r"^-\s+`([^`]+)`\s+—\s+(.+)$", re.MULTILINE)


def _parse_kcs_feature(path: Path) -> dict | None:
    """Parse a KCS feature markdown file into a structured dict."""
    try:
        content = path.read_text(encoding="utf-8")
    except OSError:
        return None

    title_match = _TITLE_RE.search(content)
    if not title_match:
        return None

    result: dict = {
        "file": path.name,
        "name": title_match.group(1).strip(),
    }

    sections = re.split(r"^##\s+", content, flags=re.MULTILINE)
    for section in sections[1:]:
        lines = section.strip().split("\n", 1)
        heading = lines[0].strip()
        body = lines[1].strip() if len(lines) > 1 else ""
        key = heading.lower().replace(" ", "_").replace("/", "_").replace("-", "_")
        result[key] = body

    return result


def _extract_screenshots(feature: dict) -> list[tuple[str, str]]:
    """Extract (ref_id, caption) pairs from the screenshots section."""
    text = feature.get("screenshots", "")
    if not text:
        return []
    return _SCREENSHOT_RE.findall(text)


def _read_screenshot(ref_id: str) -> tuple[bytes, str] | None:
    """Read a screenshot file by reference ID, trying .png then .gif."""
    for ext in (".png", ".gif"):
        path = _SCREENSHOTS_DIR / f"{ref_id}{ext}"
        if path.exists():
            mime = mimetypes.guess_type(str(path))[0] or f"image/{ext[1:]}"
            return path.read_bytes(), mime
    return None


def _surface_category(surfaces: list[str]) -> str:
    """Map surface tags to human-readable category names."""
    surface_set = set(surfaces)
    if "all-editors" in surface_set or "lsp" in surface_set:
        if "mcp" in surface_set or "claude-code" in surface_set:
            return "LSP + AI Features"
        return "LSP Features (all editors)"
    if "vscode-chat" in surface_set:
        return "VS Code AI Chat"
    if "claude-code" in surface_set:
        return "Claude Code Skills"
    if "mcp" in surface_set:
        return "MCP Tools"
    if "vscode-command" in surface_set:
        return "VS Code Commands"
    return "Other"


def build_database(out_path: Path, *, verbose: bool = False) -> None:
    """Build the KCS help SQLite database."""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    if out_path.exists():
        out_path.unlink()

    conn = sqlite3.connect(str(out_path))
    conn.execute("PRAGMA journal_mode=DELETE")

    # Features table with FTS5
    conn.execute(
        """
        CREATE VIRTUAL TABLE kcs_features USING fts5(
            name,
            summary,
            surface,
            category,
            how_to_use,
            content,
            file,
            tokenize='porter unicode61'
        )
        """
    )

    # Screenshots table
    conn.execute(
        """
        CREATE TABLE screenshots (
            kcs_file  TEXT NOT NULL,
            ref_id    TEXT NOT NULL,
            caption   TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            data      BLOB NOT NULL,
            PRIMARY KEY (kcs_file, ref_id)
        )
        """
    )

    feature_paths = sorted(_FEATURES_DIR.glob("kcs-feature-*.md"))
    if not feature_paths:
        print(f"WARN: no KCS feature files found in {_FEATURES_DIR}", file=sys.stderr)

    n_features = 0
    n_screenshots = 0

    for path in feature_paths:
        feature = _parse_kcs_feature(path)
        if feature is None:
            if verbose:
                print(f"  SKIP {path.name} (no title match)", file=sys.stderr)
            continue

        surfaces = [s.strip() for s in feature.get("surface", "").split(",")]
        category = _surface_category(surfaces)

        # Build full content blob for deep FTS matching
        content_parts = []
        for key in ("how_to_use", "operational_context", "failure_modes", "file_path_anchors"):
            val = feature.get(key, "")
            if val:
                content_parts.append(val)
        content = "\n\n".join(content_parts)

        conn.execute(
            "INSERT INTO kcs_features (name, summary, surface, category, how_to_use, content, file) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                feature.get("name", ""),
                feature.get("summary", ""),
                feature.get("surface", ""),
                category,
                feature.get("how_to_use", ""),
                content,
                feature.get("file", ""),
            ),
        )
        n_features += 1

        # Insert screenshots
        for ref_id, caption in _extract_screenshots(feature):
            screenshot = _read_screenshot(ref_id)
            if screenshot is None:
                if verbose:
                    print(f"  WARN: screenshot {ref_id} not found for {path.name}", file=sys.stderr)
                # Insert with empty blob so the reference is preserved
                conn.execute(
                    "INSERT OR IGNORE INTO screenshots (kcs_file, ref_id, caption, mime_type, data) "
                    "VALUES (?, ?, ?, ?, ?)",
                    (feature["file"], ref_id, caption, "", b""),
                )
                continue
            data, mime = screenshot
            conn.execute(
                "INSERT OR IGNORE INTO screenshots (kcs_file, ref_id, caption, mime_type, data) "
                "VALUES (?, ?, ?, ?, ?)",
                (feature["file"], ref_id, caption, mime, data),
            )
            n_screenshots += 1

    conn.commit()

    # Optimise FTS index
    conn.execute("INSERT INTO kcs_features(kcs_features) VALUES('optimize')")
    conn.commit()
    conn.execute("VACUUM")
    conn.close()

    size_kb = out_path.stat().st_size / 1024
    print(
        f"kcs_help.db: {n_features} features, {n_screenshots} screenshots, "
        f"{size_kb:.0f} KB → {out_path}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Build KCS help database")
    parser.add_argument(
        "--out",
        type=Path,
        default=_DEFAULT_OUT,
        help=f"Output path (default: {_DEFAULT_OUT})",
    )
    parser.add_argument("-v", "--verbose", action="store_true")
    args = parser.parse_args()

    build_database(args.out, verbose=args.verbose)


if __name__ == "__main__":
    main()

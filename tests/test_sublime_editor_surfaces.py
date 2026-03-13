"""Sublime integration surface checks for Tcl editor commands."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _load_json(rel_path: str):
    return json.loads((ROOT / rel_path).read_text(encoding="utf-8"))


def test_command_palette_exposes_minify_and_unminify() -> None:
    commands = _load_json("editors/sublime-text/Default.sublime-commands")
    names = {item.get("command") for item in commands}
    assert "tcl_minify_document" in names
    assert "tcl_unminify_error" in names


def test_context_menu_exposes_minify_and_unminify() -> None:
    menu = _load_json("editors/sublime-text/Context.sublime-menu")
    names = {item.get("command") for item in menu}
    assert "tcl_minify_document" in names
    assert "tcl_unminify_error" in names

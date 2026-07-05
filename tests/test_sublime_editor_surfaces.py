# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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

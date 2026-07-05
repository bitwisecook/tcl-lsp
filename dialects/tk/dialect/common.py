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

"""Shared Tk command/path constants and helpers."""

from __future__ import annotations

import re

WIDGET_COMMANDS = frozenset(
    {
        "button",
        "label",
        "entry",
        "text",
        "frame",
        "canvas",
        "listbox",
        "scrollbar",
        "menu",
        "menubutton",
        "toplevel",
        "message",
        "scale",
        "spinbox",
        "checkbutton",
        "radiobutton",
        "labelframe",
        "panedwindow",
        "destroy",
        "ttk::button",
        "ttk::label",
        "ttk::entry",
        "ttk::frame",
        "ttk::checkbutton",
        "ttk::radiobutton",
        "ttk::scrollbar",
        "ttk::spinbox",
        "ttk::scale",
        "ttk::panedwindow",
        "ttk::notebook",
        "ttk::treeview",
        "ttk::combobox",
        "ttk::progressbar",
        "ttk::separator",
        "ttk::labelframe",
        "ttk::menubutton",
        "ttk::sizegrip",
    }
)

GEOMETRY_COMMANDS = frozenset({"pack", "grid", "place"})

WIDGET_PATH_RE = re.compile(r"^\.[a-zA-Z_][a-zA-Z0-9_.]*$")


def is_widget_path(path: str) -> bool:
    """Return True if *path* matches Tcl/Tk widget path syntax."""
    return bool(WIDGET_PATH_RE.fullmatch(path))


def parent_widget_path(widget_path: str) -> str:
    """Return the parent widget path."""
    if widget_path == ".":
        return ""
    idx = widget_path.rfind(".")
    if idx <= 0:
        return "."
    return widget_path[:idx]

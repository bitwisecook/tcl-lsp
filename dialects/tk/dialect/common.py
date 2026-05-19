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

"""Extract Tk widget tree from source code for preview rendering.

Parses Tk source to build a JSON-serialisable widget hierarchy including
widget types, geometry manager assignments, and key visual properties.
"""

from __future__ import annotations

from typing import Any

from .common import (
    GEOMETRY_COMMANDS as _GEOMETRY_COMMANDS,
)
from .common import (
    WIDGET_COMMANDS as _WIDGET_COMMANDS,
)
from .common import (
    is_widget_path as _is_widget_path,
)
from .common import (
    parent_widget_path as _parent_path,
)

# Options that carry visual information for the preview.
_VISUAL_OPTIONS = frozenset(
    {
        "-text",
        "-textvariable",
        "-width",
        "-height",
        "-relief",
        "-label",
        "-orient",
        "-state",
        "-show",
        "-wrap",
        "-from",
        "-to",
        "-values",
        "-selectmode",
    }
)

_GEOMETRY_OPTIONS = frozenset(
    {
        "-row",
        "-column",
        "-rowspan",
        "-columnspan",
        "-sticky",
        "-side",
        "-fill",
        "-expand",
        "-anchor",
        "-padx",
        "-pady",
        "-ipadx",
        "-ipady",
        "-x",
        "-y",
        "-relx",
        "-rely",
        "-relwidth",
        "-relheight",
        "-in",
        "-before",
        "-after",
        "-weight",
        "-minsize",
    }
)


def _parse_options(words: list[str]) -> dict[str, str]:
    """Parse -option value pairs from a word list."""
    opts: dict[str, str] = {}
    i = 0
    while i < len(words):
        word = words[i]
        if word.startswith("-") and i + 1 < len(words):
            opts[word] = words[i + 1]
            i += 2
        else:
            i += 1
    return opts


def _short_type(cmd_name: str) -> str:
    """Shorten command name to widget type for display."""
    if cmd_name.startswith("ttk::"):
        return "ttk-" + cmd_name[5:]
    return cmd_name


def extract_tk_layout(source: str) -> dict[str, Any]:
    """Parse Tk source and extract the widget hierarchy for preview.

    Returns a JSON-serialisable dict with:
    - ``root``: The widget tree rooted at ``.`` (toplevel)
    - ``widget_count``: Total number of widgets found
    - ``geometry_conflicts``: List of geometry manager conflict messages
    """
    lines = source.splitlines()

    # Build widget info from source lines.
    widgets: dict[str, dict[str, Any]] = {
        ".": {
            "pathname": ".",
            "type": "toplevel",
            "options": {},
            "geometry": None,
            "geometry_options": {},
            "children": [],
        }
    }

    geometry_by_parent: dict[str, set[str]] = {}
    geometry_conflicts: list[str] = []

    # Extract wm title for the root window.
    title = "Tk"
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("wm title ."):
            # Extract the title string.
            parts = stripped.split(None, 3)
            if len(parts) >= 4:
                title = parts[3].strip('"').strip("{").strip("}")

    widgets["."]["title"] = title

    # Walk source lines to find widget creation and geometry commands.
    for line_text in lines:
        stripped = line_text.strip()
        if not stripped or stripped.startswith("#"):
            continue

        words = stripped.split()
        if not words:
            continue

        cmd = words[0]

        # Widget creation.
        if cmd in _WIDGET_COMMANDS and len(words) >= 2:
            path = words[1]
            if not _is_widget_path(path):
                continue

            opts = _parse_options(words[2:])
            visual_opts = {k: v for k, v in opts.items() if k in _VISUAL_OPTIONS}

            widget_info: dict[str, Any] = {
                "pathname": path,
                "type": _short_type(cmd),
                "options": visual_opts,
                "geometry": None,
                "geometry_options": {},
                "children": [],
            }
            widgets[path] = widget_info

            # Add to parent's children.
            parent = _parent_path(path)
            if parent in widgets:
                widgets[parent]["children"].append(widget_info)

        # Geometry manager commands.
        if cmd in _GEOMETRY_COMMANDS and len(words) >= 2:
            # Handle subcommands like "pack configure .w".
            target_idx = 1
            if len(words) >= 3 and words[1] in (
                "configure",
                "forget",
                "info",
                "propagate",
                "slaves",
                "columnconfigure",
                "rowconfigure",
                "remove",
                "anchor",
            ):
                target_idx = 2

            if target_idx >= len(words):
                continue

            path = words[target_idx]
            if not _is_widget_path(path):
                continue

            if path in widgets:
                geo_opts = _parse_options(words[target_idx + 1 :])
                geo_filtered = {k: v for k, v in geo_opts.items() if k in _GEOMETRY_OPTIONS}
                widgets[path]["geometry"] = cmd
                widgets[path]["geometry_options"] = geo_filtered

            # Track geometry manager per parent for conflict detection.
            parent = _parent_path(path)
            if parent not in geometry_by_parent:
                geometry_by_parent[parent] = set()
            geometry_by_parent[parent].add(cmd)

    # Detect geometry conflicts.
    for parent, managers in geometry_by_parent.items():
        if "pack" in managers and "grid" in managers:
            geometry_conflicts.append(f"Cannot mix 'pack' and 'grid' in parent '{parent}'.")

    return {
        "root": widgets["."],
        "widget_count": len(widgets),
        "geometry_conflicts": geometry_conflicts,
    }

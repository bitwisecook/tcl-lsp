"""Resolve ``source`` command targets to file URIs.

Used to build the file dependency graph for cross-file analysis.
"""

from __future__ import annotations

import os
import re

# Common Tcl idiom: source [file join [file dirname [info script]] ...]
_FILE_JOIN_INFO_SCRIPT_RE = re.compile(
    r"\[?file\s+join\s+\[file\s+dirname\s+\[info\s+script\]\]\s+(.+?)\]?$"
)


def resolve_source_target(
    raw_path: str,
    is_literal: bool,
    script_path: str,
    workspace_roots: list[str] | None = None,
) -> str | None:
    """Resolve a ``source`` target to an absolute file path.

    Returns the resolved path, or ``None`` if unresolvable.

    Parameters
    ----------
    raw_path:
        The literal text of the source command's file argument.
    is_literal:
        ``False`` when the path contains ``$`` or ``[`` substitutions.
    script_path:
        Absolute path of the file containing the ``source`` command.
    workspace_roots:
        Workspace root directories to try for relative paths.
    """
    if not is_literal:
        # Try to extract a relative path from the [file join ...] idiom.
        m = _FILE_JOIN_INFO_SCRIPT_RE.search(raw_path)
        if m:
            rel = m.group(1).strip()
            # The relative portion itself may still contain substitutions.
            if "$" in rel or "[" in rel:
                return None
            script_dir = os.path.dirname(script_path)
            candidate = os.path.normpath(os.path.join(script_dir, rel))
            if os.path.isfile(candidate):
                return candidate
        return None

    # Literal path — try relative to the script's own directory first.
    script_dir = os.path.dirname(script_path)
    candidate = os.path.normpath(os.path.join(script_dir, raw_path))
    if os.path.isfile(candidate):
        return candidate

    # Try relative to each workspace root.
    for root in workspace_roots or []:
        candidate = os.path.normpath(os.path.join(root, raw_path))
        if os.path.isfile(candidate):
            return candidate

    # Absolute path?
    if os.path.isabs(raw_path) and os.path.isfile(raw_path):
        return raw_path

    return None

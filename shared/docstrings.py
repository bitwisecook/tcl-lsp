"""Tcl-text-generic docstring helpers (leaf utilities).

Lives in ``shared/`` because both the IDE analyser (proc-doc semantic
enrichment) and the developer formatter (docstring rendering) need to
pull the leading comment block off a proc body, and ``shared/`` is the
only place both may import from without crossing a layering contract.
"""

from __future__ import annotations

# Characters that, on their own, make a comment line pure decoration
# (``# -----`` / ``# .....`` / ``# =====``) rather than documentation.
_DECORATION_CHARS = frozenset(".-=*~#")


def extract_body_docstring(body: str) -> str:
    """Extract the leading comment block from a proc body.

    Returns the accumulated comment text (lines joined with newlines) if
    the body starts with one or more comment lines, otherwise returns an
    empty string.  Decoration lines consisting only of dots, dashes,
    hashes, or similar characters are stripped.
    """
    lines: list[str] = []
    for raw_line in body.splitlines():
        stripped = raw_line.strip()
        if not stripped:
            if lines:
                break
            continue
        if stripped.startswith("#"):
            text = stripped.lstrip("#").strip()
            # Skip hash-only decoration lines
            if not text and set(stripped) <= {"#"}:
                continue
            # Skip decoration lines (dots, dashes, equals, etc.)
            if text and all(ch in _DECORATION_CHARS for ch in text):
                continue
            lines.append(text)
        else:
            break
    return "\n".join(lines)

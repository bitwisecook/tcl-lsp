"""Locate and extract embedded Tcl bodies from ``sys application template`` blocks.

Extracts ``implementation`` and ``presentation`` sections from iApp
templates in BIG-IP configuration files so they can receive full Tcl
semantic tokenisation.
"""

from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class EmbeddedSection:
    """A Tcl code section inside an iApp template."""

    kind: str  # "implementation" or "presentation"
    body: str  # the Tcl source between the outermost { }
    body_start_offset: int  # offset of the first char after the opening {
    body_end_offset: int  # offset of the last char before the closing }


# Match:  implementation {  or  presentation {
_SECTION_RE = re.compile(
    r"^\s+(implementation|presentation)\s*\{",
    re.MULTILINE,
)


def _brace_match(source: str, brace_pos: int) -> int:
    """Return the offset past the closing ``}`` that matches the ``{`` at *brace_pos*."""
    pos = brace_pos + 1
    depth = 1
    while pos < len(source) and depth > 0:
        ch = source[pos]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif ch == "\\":
            pos += 1  # skip escaped char
        pos += 1
    return pos


def find_embedded_iapp_sections(source: str) -> list[EmbeddedSection]:
    """Find ``implementation`` and ``presentation`` bodies in iApp templates."""
    sections: list[EmbeddedSection] = []
    for m in _SECTION_RE.finditer(source):
        kind = m.group(1)
        brace_pos = m.end() - 1  # position of '{'
        end_pos = _brace_match(source, brace_pos)
        body = source[brace_pos + 1 : end_pos - 1]
        # Only include sections that actually contain Tcl code.
        if body.strip():
            sections.append(
                EmbeddedSection(
                    kind=kind,
                    body=body,
                    body_start_offset=brace_pos + 1,
                    body_end_offset=end_pos - 1,
                )
            )
    return sections

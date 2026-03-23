"""Shared text-processing helpers for iApp/APL modules."""

from __future__ import annotations


def find_brace_end(source: str, start: int) -> int:
    """Return offset past the closing ``}`` matching the ``{`` at *start*.

    Handles braces inside double-quoted strings so that a ``"}"`` literal
    does not prematurely close the block.
    """
    pos = start + 1
    depth = 1
    while pos < len(source) and depth > 0:
        ch = source[pos]
        if ch == '"':
            # Skip entire quoted string (may contain braces)
            pos += 1
            while pos < len(source) and source[pos] != '"':
                if source[pos] == "\\":
                    pos += 1  # skip escaped char inside string
                pos += 1
            # Advance past closing quote
            pos += 1
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif ch == "\\":
            pos += 1  # skip escaped char
        pos += 1
    return pos


def offset_to_line_char(source: str, offset: int) -> tuple[int, int]:
    """Convert a byte offset to (line, character) pair."""
    line = source.count("\n", 0, offset)
    line_start = source.rfind("\n", 0, offset) + 1
    return (line, offset - line_start)

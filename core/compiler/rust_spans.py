"""Shared helpers for converting Rust-side byte offsets to Python
:class:`SourcePosition` / :class:`Range` values.

The Rust crates emit spans as UTF-8 byte offsets with exclusive-end
semantics. Python's LSP-facing :class:`SourcePosition` carries
line, LSP-UTF-16 column, and (absolute) offset, with
:class:`Range` storing inclusive-end positions. Multiple Rust→Python
binding shims need the same conversion; centralising the logic
here avoids silent divergence (especially around UTF-16 column
math and the exclusive/inclusive boundary).
"""

from __future__ import annotations

import bisect
import os

from ..analysis.semantic_model import Range
from ..parsing.tokens import SourcePosition

# Truthy values that opt a `TCL_LSP_RUST_*` shim into the Rust path.
# `os.environ.get(name)` alone treats any non-empty string as truthy
# (including `"0"` / `"false"`), which contradicts the docstring
# convention of `=1` and silently opts users into the experimental
# path when they thought they were turning it off.
_TRUTHY_VALUES = frozenset({"1", "true", "yes", "on", "y", "t"})


def rust_shim_enabled(env_var: str) -> bool:
    """Return ``True`` when the named ``TCL_LSP_RUST_*`` env var is
    set to a truthy value.

    Treats `"1"`, `"true"`, `"yes"`, `"on"`, `"y"`, `"t"`
    (case-insensitive, after stripping whitespace) as truthy. Any
    other value — including an unset variable, the empty string,
    `"0"`, `"false"`, `"no"`, `"off"` — counts as off.

    Centralising the check here prevents silent divergence between
    the four `TCL_LSP_RUST_*` dispatch shims (`OPTIMISER`,
    `INTERPROC`, `GVN`, `SIGNATURE_SCAN`).
    """
    return os.environ.get(env_var, "").strip().lower() in _TRUTHY_VALUES


def build_position_resolver(source: str):
    """Return ``(position_at, range_at)`` callables for ``source``.

    Precomputes the UTF-8 byte representation and line-start byte
    offsets once. ``position_at(offset)`` returns a
    :class:`SourcePosition`; ``range_at(start, end)`` returns a
    :class:`Range`, collapsing the exclusive Rust end-offset to
    Python's inclusive-end convention (``end-1`` for non-empty
    spans, ``start`` for empty ones).

    Both callables clamp out-of-bounds offsets to the source bounds
    and use ``errors='replace'`` for any mid-codepoint slice, so
    they never raise on malformed input — they always produce *a*
    position.
    """
    utf8_source = source.encode("utf-8")
    source_len = len(utf8_source)

    # Precompute line-start byte offsets once for O(log n) lookup.
    line_starts = [0]
    for i, b in enumerate(utf8_source):
        if b == 0x0A:  # '\n'
            line_starts.append(i + 1)

    def position_at(offset: int) -> SourcePosition:
        clamped = max(0, min(offset, source_len))
        idx = bisect.bisect_right(line_starts, clamped) - 1
        if idx < 0:
            idx = 0
        line_start = line_starts[idx]
        # Decode the line prefix and measure its UTF-16 length so
        # ``character`` matches LSP semantics.
        line_prefix = utf8_source[line_start:clamped].decode("utf-8", errors="replace")
        utf16_column = len(line_prefix.encode("utf-16-le")) // 2
        return SourcePosition(line=idx, character=utf16_column, offset=clamped)

    def range_at(start: int, end: int) -> Range:
        # Rust Span is exclusive-end; Python Range.end is inclusive.
        # Subtract 1 when the span is non-empty.
        end_incl = max(start, end - 1) if end > start else start
        return Range(start=position_at(start), end=position_at(end_incl))

    return position_at, range_at

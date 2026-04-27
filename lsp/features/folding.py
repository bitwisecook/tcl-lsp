"""Folding range provider — proc bodies, namespaces, comment blocks, control structures.

The collection algorithm — scope walk, comment-line block walker, and
the registry-driven body-argument walker — lives in Rust at
``rust/tcl-lsp-rust/src/folding.rs`` and is exposed to Python via the
``tcl_lsp_rust.folding_ranges`` PyO3 entry point.  This module is a
thin dispatcher: it calls the Rust collector, materialises
``lsprotocol.types.FoldingRange`` values, and runs
:func:`_normalise_overlaps` to ensure VS Code's folding tree-builder
sees properly nested or disjoint ranges.

:func:`_normalise_overlaps` stays in Python: ``tests/test_folding.py``
exercises it directly with hand-built ranges.
"""

from __future__ import annotations

from lsprotocol import types

from core.analysis.semantic_model import AnalysisResult
from core.commands.registry.runtime import active_signature_profile

try:
    from tcl_lsp_rust import (  # ty: ignore[unresolved-import]
        folding_ranges as _rust_folding_ranges,
    )
except ImportError:  # pragma: no cover - rust binding is optional at typecheck time
    _rust_folding_ranges = None


# Wire forms emitted by the Rust binding (lower-case to mirror the
# `lsprotocol.types.FoldingRangeKind` enum's string values).
_KIND_TO_LSP: dict[str, types.FoldingRangeKind] = {
    "region": types.FoldingRangeKind.Region,
    "comment": types.FoldingRangeKind.Comment,
}


def _normalise_overlaps(
    ranges: list[types.FoldingRange],
) -> list[types.FoldingRange]:
    """Ensure folding ranges are disjoint or properly nested.

    VS Code builds a folding tree from the returned ranges and silently drops
    or misplaces ranges that partially overlap (share a boundary line without
    one containing the other).  The Rust collector already tries to avoid
    this via its ``adjust_body_end_line`` rule, but a belt-and-suspenders
    post-pass keeps the output well-formed even if a new collector forgets
    the invariant.

    ``FoldingRange.end_line`` is inclusive, so two ranges that share a
    boundary line (e.g. ``[0, 5]`` and ``[5, 10]``) both include the shared
    line and are neither disjoint nor strictly nested.  When that pattern
    slips past the collector, we trim the earlier sibling's end so the two
    become disjoint; when a range extends past the open parent, we trim
    the range down to the parent's end.  A final pass drops any duplicate
    ``(start, end, kind)`` triples produced by those adjustments.
    """
    if not ranges:
        return ranges

    # Sort by start ascending, end descending so that parents come before
    # children and equal-start ranges with larger spans come first.
    ordered = sorted(ranges, key=lambda r: (r.start_line, -r.end_line))

    # working[i] may be replaced in-place (to trim a previously-emitted
    # parent) or set to None to drop it outright.  stack holds indices of
    # currently-open ancestors.
    working: list[types.FoldingRange | None] = []
    stack: list[int] = []

    for r in ordered:
        # Close or trim ancestors that conflict with r's start.  Stack entries
        # always reference a live (non-None) ``working`` slot: we only set an
        # entry to None immediately before popping its index off the stack.
        while stack:
            parent = working[stack[-1]]
            if parent is None:
                # Defensive: should never happen, but keep the loop safe under
                # ``python -O`` where plain ``assert`` would be stripped.
                stack.pop()
                continue
            if parent.end_line < r.start_line:
                stack.pop()
                continue
            if parent.end_line == r.start_line:
                # Inclusive end_line: a shared boundary still overlaps, so
                # trim the parent back by one line if that leaves a useful
                # fold, otherwise drop it entirely.
                if parent.end_line - 1 > parent.start_line:
                    working[stack[-1]] = types.FoldingRange(
                        start_line=parent.start_line,
                        end_line=parent.end_line - 1,
                        kind=parent.kind,
                    )
                else:
                    working[stack[-1]] = None
                stack.pop()
                continue
            break

        # Trim r down to fit inside its (new) parent, if any.
        if stack:
            parent = working[stack[-1]]
            if parent is not None and parent.end_line < r.end_line:
                if parent.end_line <= r.start_line:
                    # Trim would leave r degenerate or inverted — drop it.
                    continue
                r = types.FoldingRange(
                    start_line=r.start_line,
                    end_line=parent.end_line,
                    kind=r.kind,
                )

        working.append(r)
        stack.append(len(working) - 1)

    # De-duplicate: trimming parents or ranges may have collapsed distinct
    # inputs onto the same (start, end, kind) triple.  ``kind`` is declared
    # by lsprotocol as ``Optional[Union[FoldingRangeKind, str]]``.
    seen: set[tuple[int, int, types.FoldingRangeKind | str | None]] = set()
    result: list[types.FoldingRange] = []
    for r in working:
        if r is None:
            continue
        key = (r.start_line, r.end_line, r.kind)
        if key in seen:
            continue
        seen.add(key)
        result.append(r)
    return result


def get_folding_ranges(
    source: str,
    analysis: AnalysisResult | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.FoldingRange]:
    """Return folding ranges for a Tcl source file.

    The ``analysis`` and ``lines`` parameters are accepted for source
    compatibility with the prior Python implementation's call sites
    (``lsp/server.py``); the Rust collector takes its own analyser
    pass internally and reads lines from the source string.  Neither
    argument is currently consulted.
    """
    if not source or _rust_folding_ranges is None:
        return []
    dialect = str(active_signature_profile().get("dialect") or "tcl8.6")
    raw = _rust_folding_ranges(source, dialect)
    materialised = [
        types.FoldingRange(
            start_line=item["start_line"],
            end_line=item["end_line"],
            kind=_KIND_TO_LSP.get(item["kind"], item["kind"]),
        )
        for item in raw
    ]
    return _normalise_overlaps(materialised)

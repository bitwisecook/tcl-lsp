"""Folding range provider — proc bodies, namespaces, comment blocks, control structures.

The Rust binding ``tcl_lsp_rust.folding_ranges`` is the fast path: it
runs the analyser, the comment-line walker, and the registry-driven
body-argument walker entirely in native code.  When the binding is
unavailable — typecheck environments uninstall the wheel
(``make typecheck-py``), zipapp builds may ship without it, fresh
checkouts before ``make rust-build`` lands — this module falls back
to a Python implementation with identical behaviour.

Both paths feed into :func:`_normalise_overlaps`, the
belt-and-suspenders pass that protects VS Code's folding
tree-builder from any partial-overlap escape hatch.
"""

from __future__ import annotations

from lsprotocol import types

from core.analysis.semantic_model import AnalysisResult, Scope
from core.commands.registry.runtime import (
    active_signature_profile,
    iter_body_arguments,
)
from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import Token, TokenType

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


def _adjust_body_end_line(source: str, end_offset: int, end_line: int) -> int:
    """Return a fold end line that leaves the closing ``}`` visible.

    A braced body token ends at the character immediately before its closing
    ``}``.  When that character is a newline, ``}`` is on ``end_line + 1`` and
    ``end_line`` is already the correct fold end (VS Code keeps the line after
    ``end_line`` visible).  When ``}`` sits on the same line as the last byte
    of content -- e.g. ``} else {`` or a trailing ``}`` on the same line as
    inner text -- the unadjusted fold would cover the separator line and
    collide with the next sibling body's fold range, producing the
    non-hierarchical overlap VS Code's folding tree-builder rejects.  Moving
    the fold end up one line in that case keeps siblings disjoint.

    Incomplete bodies (no closing ``}`` yet -- common while the user is still
    typing) don't need the adjustment: there is no separator line to preserve,
    and decrementing would collapse the fold range away mid-edit.
    """
    if end_offset < 0 or end_offset >= len(source):
        return end_line
    if source[end_offset] == "\n":
        return end_line
    # Only adjust when there is an actual closing ``}`` right after the
    # content -- otherwise the body is unterminated and the fold should span
    # what the user has so far.
    if end_offset + 1 < len(source) and source[end_offset + 1] == "}":
        return end_line - 1
    return end_line


def _collect_scope_folds(
    scope: Scope,
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
    source: str,
) -> None:
    """Emit folding ranges from the scope tree (procs and namespaces)."""
    for proc_def in scope.procs.values():
        br = proc_def.body_range
        if br.start.line < br.end.line:
            end_line = _adjust_body_end_line(source, br.end.offset, br.end.line)
            if end_line > br.start.line:
                key = (br.start.line, end_line)
                if key not in seen:
                    seen.add(key)
                    ranges.append(
                        types.FoldingRange(
                            start_line=br.start.line,
                            end_line=end_line,
                            kind=types.FoldingRangeKind.Region,
                        )
                    )

    for child in scope.children:
        if child.kind == "namespace" and child.body_range is not None:
            br = child.body_range
            if br.start.line < br.end.line:
                end_line = _adjust_body_end_line(source, br.end.offset, br.end.line)
                if end_line > br.start.line:
                    key = (br.start.line, end_line)
                    if key not in seen:
                        seen.add(key)
                        ranges.append(
                            types.FoldingRange(
                                start_line=br.start.line,
                                end_line=end_line,
                                kind=types.FoldingRangeKind.Region,
                            )
                        )
        _collect_scope_folds(child, seen, ranges, source)


def _collect_comment_folds(
    source: str,
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
    *,
    lines: list[str] | None = None,
) -> None:
    """Emit folding ranges for consecutive comment-line blocks."""
    if lines is None:
        lines = source.split("\n")
    block_start: int | None = None

    for i, line in enumerate(lines):
        stripped = line.lstrip()
        if stripped.startswith("#"):
            if block_start is None:
                block_start = i
        else:
            if block_start is not None and i - block_start >= 2:
                key = (block_start, i - 1)
                if key not in seen:
                    seen.add(key)
                    ranges.append(
                        types.FoldingRange(
                            start_line=block_start,
                            end_line=i - 1,
                            kind=types.FoldingRangeKind.Comment,
                        )
                    )
            block_start = None

    # Handle trailing comment block at end of file
    if block_start is not None:
        end = len(lines) - 1
        if end - block_start >= 1:
            key = (block_start, end)
            if key not in seen:
                seen.add(key)
                ranges.append(
                    types.FoldingRange(
                        start_line=block_start,
                        end_line=end,
                        kind=types.FoldingRangeKind.Comment,
                    )
                )


def _collect_body_folds(
    source: str,
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
    *,
    original_source: str,
    body_token: Token | None = None,
    depth: int = 0,
) -> None:
    """Recursively segment commands and emit folds for multi-line BODY args."""
    if depth > 20:
        return

    for cmd in segment_commands(source, body_token):
        if not cmd.argv:
            continue
        for body in iter_body_arguments(cmd.name, cmd.args, cmd.arg_tokens):
            if body.token.type is not TokenType.STR:
                continue
            if body.token.start.line < body.token.end.line:
                end_line = _adjust_body_end_line(
                    original_source,
                    body.token.end.offset,
                    body.token.end.line,
                )
                if end_line > body.token.start.line:
                    key = (body.token.start.line, end_line)
                    if key not in seen:
                        seen.add(key)
                        ranges.append(
                            types.FoldingRange(
                                start_line=body.token.start.line,
                                end_line=end_line,
                                kind=types.FoldingRangeKind.Region,
                            )
                        )
                # Recurse into the body regardless of whether the outer fold
                # was adjusted away -- inner multi-line bodies may still be
                # foldable even when the enclosing token collapses to a
                # single effective line.
                _collect_body_folds(
                    body.text,
                    seen,
                    ranges,
                    original_source=original_source,
                    body_token=body.token,
                    depth=depth + 1,
                )


def _normalise_overlaps(
    ranges: list[types.FoldingRange],
) -> list[types.FoldingRange]:
    """Ensure folding ranges are disjoint or properly nested.

    VS Code builds a folding tree from the returned ranges and silently drops
    or misplaces ranges that partially overlap (share a boundary line without
    one containing the other).  The collectors already try to avoid this via
    ``_adjust_body_end_line``, but a belt-and-suspenders post-pass keeps the
    output well-formed even if a new collector forgets the invariant.

    ``FoldingRange.end_line`` is inclusive, so two ranges that share a
    boundary line (e.g. ``[0, 5]`` and ``[5, 10]``) both include the shared
    line and are neither disjoint nor strictly nested.  When that pattern
    slips past the collectors, we trim the earlier sibling's end so the two
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


def _python_folding_ranges(
    source: str,
    analysis: AnalysisResult | None,
    *,
    lines: list[str] | None,
) -> list[types.FoldingRange]:
    """Pure-Python collection path used when the Rust binding is absent."""
    ranges: list[types.FoldingRange] = []
    seen: set[tuple[int, int]] = set()

    if analysis is not None:
        _collect_scope_folds(analysis.global_scope, seen, ranges, source)
    _collect_comment_folds(source, seen, ranges, lines=lines)
    _collect_body_folds(source, seen, ranges, original_source=source)

    return ranges


def get_folding_ranges(
    source: str,
    analysis: AnalysisResult | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.FoldingRange]:
    """Return folding ranges for a Tcl source file.

    Prefers the Rust collector via ``tcl_lsp_rust.folding_ranges``;
    falls back to the Python collectors when the binding isn't
    available.  Both paths run through :func:`_normalise_overlaps`
    before returning.

    When the Rust path runs the ``analysis`` and ``lines`` keyword
    arguments are unused — the Rust collector takes its own analyser
    pass internally and reads lines from the source string.  The
    Python fallback honours both arguments for the same behaviour
    the prior implementation had: ``analysis=None`` skips the
    scope-tree walk, and ``lines`` lets callers pass a pre-split
    line list to avoid one ``str.split`` per request.
    """
    if not source:
        return []
    if _rust_folding_ranges is not None:
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
    return _normalise_overlaps(_python_folding_ranges(source, analysis, lines=lines))

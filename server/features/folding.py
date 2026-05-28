"""Folding range provider.

Emits whole-line folding ranges (VS Code advertises ``lineFoldingOnly`` so
``startCharacter`` / ``endCharacter`` are ignored).  ``FoldingRange.end_line``
is *inclusive*; ``start_line`` stays visible when the range is collapsed.

Folds produced, in priority order (the first collector to claim a line span
wins, so later collectors never double-fold the same lines):

* **region markers** -- ``#region`` / ``#endregion`` comment pairs (Region).
* **blocks** -- any multi-line braced ``{...}`` body or data literal,
  bracketed ``[...]`` command substitution, and quoted ``"..."`` string
  (Region).  This covers proc / namespace / control-flow bodies *and* plain
  multi-line lists, dicts, ``switch`` arms, and strings uniformly.
* **import groups** -- runs of ``package require`` / ``source`` / ``load``
  lines (Imports).
* **comment blocks** -- runs of ``#`` comment lines, incl. license headers
  (Comment).
* **line continuations** -- commands stretched across lines by a trailing
  ``\\`` (Region).
"""

from __future__ import annotations

import re
from collections.abc import Sequence
from typing import TYPE_CHECKING

from lsprotocol import types

from compiler.parsing.green_tree import tokenise
from shared.tokens import Token, TokenType

if TYPE_CHECKING:
    from analyser.semantic_model import AnalysisResult

# Lexer text for a backslash-newline line continuation: a literal ``\``
# followed by the newline it joins across.
_CONTINUATION = "\\\n"

# Characters that close a foldable block on the *same* line as its last byte
# of content (``}`` brace body, ``]`` command substitution, ``"`` string).
_BLOCK_CLOSERS = '}]"'

# Region markers, matching the regexes VS Code's language-configuration uses
# (``^\s*#\s*region\b`` / ``^\s*#\s*endregion\b``).  Emitting them from the
# server makes region folding work in editors that don't read the VS Code
# language-configuration (Zed, Neovim, Emacs, Sublime, Helix).
_REGION_START = re.compile(r"^\s*#\s*region\b")
_REGION_END = re.compile(r"^\s*#\s*endregion\b")

# Import-like commands whose consecutive runs fold as an ``imports`` group.
_IMPORT_RE = re.compile(r"^\s*(?:package\s+require|source|load)\b")


def _adjust_block_end_line(source: str, end_offset: int, end_line: int) -> int:
    """Return a fold end line that leaves a block's closing delimiter visible.

    A braced/bracketed/quoted token ends at the character immediately before
    its closer.  When that character is a newline the closer sits on
    ``end_line + 1`` and ``end_line`` is already correct (VS Code keeps the
    line after ``end_line`` visible).  When the closer sits on the same line
    as the last byte of content -- e.g. ``} else {`` or a trailing ``}`` /
    ``]`` / ``"`` on the same line as inner text -- the unadjusted fold would
    cover the separator line and collide with the next sibling's fold range,
    producing the non-hierarchical overlap VS Code's folding tree-builder
    rejects.  Moving the fold end up one line keeps siblings disjoint.

    Incomplete blocks (no closer yet -- common while the user is still typing)
    don't need the adjustment: there is no separator line to preserve, and
    decrementing would collapse the fold range away mid-edit.
    """
    if end_offset < 0 or end_offset >= len(source):
        return end_line
    if source[end_offset] == "\n":
        return end_line
    if end_offset + 1 < len(source) and source[end_offset + 1] in _BLOCK_CLOSERS:
        return end_line - 1
    return end_line


def _emit(
    start_line: int,
    end_line: int,
    kind: types.FoldingRangeKind,
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
) -> None:
    """Append a fold if it spans >1 line and no fold already claims its span."""
    if end_line <= start_line:
        return
    key = (start_line, end_line)
    if key in seen:
        return
    seen.add(key)
    ranges.append(types.FoldingRange(start_line=start_line, end_line=end_line, kind=kind))


def _is_quoted_string(full_source: str, tok: Token) -> bool:
    """Whether *tok* is a double-quoted word (its span opens with ``"``)."""
    start = tok.start.offset
    return 0 <= start < len(full_source) and full_source[start] == '"'


def _collect_block_folds(
    full_source: str,
    source: str,
    base_offset: int,
    base_line: int,
    base_col: int,
    lines: list[str],
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
    *,
    depth: int = 0,
) -> None:
    """Fold every multi-line braced/bracketed/quoted token, recursively.

    Walks the token stream and emits a Region fold for each ``{...}`` brace
    word (proc / namespace / control-flow body *or* a data literal such as a
    list or dict), each ``[...]`` command substitution, and each ``"..."``
    string that spans more than one line.  Recurses into the contents of brace
    and bracket tokens so nested bodies / literals also fold.  ``full_source``
    is always the original document text -- token offsets are absolute, so
    delimiter look-ahead and recursion anchoring index into it directly.

    Backslash-newline continuation folds are emitted from this same walk:
    each level's token stream surfaces the ``SEP`` join tokens at that nesting
    depth, so a ``\\``-continued command inside a proc / ``if`` / namespace
    body folds just like a top-level one (the inner ``SEP`` tokens are only
    visible once we recurse into the body, where the enclosing braces are no
    longer a single opaque ``STR`` token).
    """
    if depth > 25:
        return
    tokens, _ = tokenise(source, base_offset, base_line, base_col)
    _emit_continuation_runs(tokens, lines, seen, ranges)
    for tok in tokens:
        if tok.start.line >= tok.end.line:
            continue
        if tok.type in (TokenType.STR, TokenType.CMD):
            end_line = _adjust_block_end_line(full_source, tok.end.offset, tok.end.line)
            _emit(tok.start.line, end_line, types.FoldingRangeKind.Region, seen, ranges)
            # Recurse into the body content regardless of whether the outer
            # fold survived -- an inner multi-line block may still be foldable
            # when the enclosing token collapses to a single effective line.
            # Content begins one character past the opening delimiter, which is
            # where the lexer anchors a STR/CMD body (mirrors the command
            # segmenter's base-position logic).
            _collect_block_folds(
                full_source,
                tok.text,
                tok.start.offset + 1,
                tok.start.line,
                tok.start.character + 1,
                lines,
                seen,
                ranges,
                depth=depth + 1,
            )
        elif tok.type is TokenType.ESC and _is_quoted_string(full_source, tok):
            end_line = _adjust_block_end_line(full_source, tok.end.offset, tok.end.line)
            _emit(tok.start.line, end_line, types.FoldingRangeKind.Region, seen, ranges)


def _collect_region_folds(
    lines: list[str],
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
) -> None:
    """Fold ``#region`` / ``#endregion`` marker pairs (nesting-aware)."""
    stack: list[int] = []
    for i, line in enumerate(lines):
        if _REGION_END.match(line):
            if stack:
                start = stack.pop()
                _emit(start, i, types.FoldingRangeKind.Region, seen, ranges)
        elif _REGION_START.match(line):
            stack.append(i)


def _collect_import_folds(
    lines: list[str],
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
) -> None:
    """Fold consecutive ``package require`` / ``source`` / ``load`` runs."""
    block_start: int | None = None
    for i, line in enumerate(lines):
        if _IMPORT_RE.match(line):
            if block_start is None:
                block_start = i
        else:
            if block_start is not None:
                _emit(block_start, i - 1, types.FoldingRangeKind.Imports, seen, ranges)
            block_start = None
    if block_start is not None:
        _emit(block_start, len(lines) - 1, types.FoldingRangeKind.Imports, seen, ranges)


def _collect_comment_folds(
    lines: list[str],
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
) -> None:
    """Fold runs of consecutive ``#`` comment lines (>=2 lines).

    Region markers (``#region`` / ``#endregion``) break a comment run so they
    surface as Region folds rather than being absorbed into a comment block.
    """

    def is_comment(line: str) -> bool:
        stripped = line.lstrip()
        if not stripped.startswith("#"):
            return False
        return not (_REGION_START.match(line) or _REGION_END.match(line))

    block_start: int | None = None
    for i, line in enumerate(lines):
        if is_comment(line):
            if block_start is None:
                block_start = i
        else:
            if block_start is not None:
                _emit(block_start, i - 1, types.FoldingRangeKind.Comment, seen, ranges)
            block_start = None
    if block_start is not None:
        _emit(block_start, len(lines) - 1, types.FoldingRangeKind.Comment, seen, ranges)


def _emit_continuation_runs(
    tokens: Sequence[Token],
    lines: list[str],
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
) -> None:
    """Fold commands stretched across lines by ``\\``-continuations.

    A command such as ::

        MyProcCall $a \\
                   $b \\
                   $c

    is a single logical command spread over several physical lines.  The lexer
    represents each backslash-newline join between words as a ``SEP`` token
    whose text is exactly ``"\\\\\\n"``; a token starting on line *L* means line
    *L* continues onto line *L + 1*.  Consecutive continued lines form one run,
    so a fold spanning the run collapses the whole command down to its first
    line (the behaviour requested in #493).

    *tokens* is one nesting level's token stream (the whole document at the top
    level, or a braced/bracketed body's contents when called from the recursive
    block walk), so continuations fold at every depth.
    """
    continued = sorted(
        {
            tok.start.line
            for tok in tokens
            if tok.type is TokenType.SEP and tok.text == _CONTINUATION
        }
    )
    if not continued:
        return

    run_start = prev = continued[0]
    for line in continued[1:]:
        if line == prev + 1:
            prev = line
            continue
        _emit_continuation_run(run_start, prev, lines, seen, ranges)
        run_start = prev = line
    _emit_continuation_run(run_start, prev, lines, seen, ranges)


def _emit_continuation_run(
    run_start: int,
    run_end: int,
    lines: list[str],
    seen: set[tuple[int, int]],
    ranges: list[types.FoldingRange],
) -> None:
    """Emit a fold for a run of continued lines ``run_start..run_end``.

    Each line in the run ends with a continuation, so the command's final
    physical line is ``run_end + 1`` -- the line the last ``\\`` joins onto.
    A trailing backslash with no following content (``foo \\`` at end of file)
    points past the last real line; trim blank trailing lines so a dangling
    continuation doesn't produce a degenerate fold over empty space.
    """
    end_line = run_end + 1
    while end_line > run_start and (end_line >= len(lines) or not lines[end_line].strip()):
        end_line -= 1
    _emit(run_start, end_line, types.FoldingRangeKind.Region, seen, ranges)


def _normalise_overlaps(
    ranges: list[types.FoldingRange],
) -> list[types.FoldingRange]:
    """Ensure folding ranges are disjoint or properly nested.

    VS Code builds a folding tree from the returned ranges and silently drops
    or misplaces ranges that partially overlap (share a boundary line without
    one containing the other).  The collectors already try to avoid this via
    ``_adjust_block_end_line``, but a belt-and-suspenders post-pass keeps the
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


def get_folding_ranges(
    source: str,
    analysis: AnalysisResult | None = None,
    *,
    lines: list[str] | None = None,
) -> list[types.FoldingRange]:
    """Return folding ranges for a Tcl source file.

    Folding is purely syntactic -- it works on source tokens and lines alone,
    so it serves results immediately after ``didOpen`` without waiting on the
    background analyse() task.  ``analysis`` is accepted for call-site
    compatibility but is no longer needed: the block collector folds proc /
    namespace / control-flow bodies (and every other multi-line brace) without
    a semantic model.
    """
    if lines is None:
        lines = source.split("\n")
    ranges: list[types.FoldingRange] = []
    seen: set[tuple[int, int]] = set()

    _collect_region_folds(lines, seen, ranges)
    # The block walk also emits continuation folds at every nesting level.
    _collect_block_folds(source, source, 0, 0, 0, lines, seen, ranges)
    _collect_import_folds(lines, seen, ranges)
    _collect_comment_folds(lines, seen, ranges)

    return _normalise_overlaps(ranges)

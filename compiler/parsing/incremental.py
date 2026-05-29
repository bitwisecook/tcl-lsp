"""Incremental reparse: edit-range inference and chunk-level tree update.

pygls is configured for FULL text sync, so ``did_change`` delivers the entire
new source rather than edit ranges.  :func:`infer_edit_range` recovers the
changed span by a common-prefix / common-suffix diff, and
:func:`incremental_top_level_chunks` rebuilds the top-level chunk list for the
new source by reusing the previous segmentation everywhere the edit did not
reach:

- chunks entirely **before** the edit are reused verbatim (same offsets,
  hashes, and commands);
- chunks entirely **after** the edit, on lines below the edit's last line,
  are reused with their offsets/lines shifted by the edit's deltas (columns
  are unchanged because those tokens sit on later lines);
- only the **middle** window straddling the edit is re-tokenised.

This is the green tree's incremental reparse realised at the top-level-command
granularity the rest of the pipeline already caches at (see
``docs/design/compiler/green-token-tree.md`` section D).  The result is
byte-identical to a from-scratch :func:`segment_top_level_chunks`; a property
test enforces that, and any case the fast path cannot prove equivalent (error
recovery, partial commands, no clean suffix) returns ``None`` so the caller
falls back to a full re-segmentation.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from shared.document_buffer import DocumentBuffer
from shared.tokens import Token, TokenType

from .command_segmenter import (
    SegmentedCommand,
    TopLevelChunk,
    segment_commands,
    tile_commands,
)
from .token_positions import shift_range, shift_token

if TYPE_CHECKING:
    from collections.abc import Sequence


@dataclass(frozen=True, slots=True)
class EditRange:
    """A single contiguous edit inferred from old/new full source.

    The edit replaces ``old[start:old_end]`` with ``new[start:new_end]``;
    ``old[old_end:] == new[new_end:]`` and ``old[:start] == new[:start]``.
    """

    start: int
    old_end: int
    new_end: int

    @property
    def offset_delta(self) -> int:
        """Byte shift applied to everything at/after ``old_end``."""
        return self.new_end - self.old_end


# Window size for the block-wise prefix/suffix scans below.  Each step compares
# at most this many characters per side (a single C-speed string compare), so
# peak allocation is bounded regardless of document size.
_SCAN_BLOCK = 4096


def _common_prefix_len(a: str, b: str, hi: int) -> int:
    """Largest ``k`` in ``[0, hi]`` with ``a[:k] == b[:k]``.

    Block-wise forward scan: compare bounded ``_SCAN_BLOCK``-sized windows (each
    a single C-speed string compare) until one differs, then pin down the exact
    divergence char-by-char within that block.  This runs on every keystroke, so
    it avoids the old binary search whose every probe sliced an ``O(mid)`` whole
    prefix — allocating up to ``a[:hi // 2]`` on the very first comparison.
    """
    i = 0
    while i < hi:
        end = min(i + _SCAN_BLOCK, hi)
        if a[i:end] == b[i:end]:
            i = end
            continue
        while i < end and a[i] == b[i]:
            i += 1
        return i
    return hi


def _common_suffix_len(a: str, b: str, hi: int) -> int:
    """Largest ``k`` in ``[0, hi]`` with ``a[len(a)-k:] == b[len(b)-k:]``.

    Block-wise backward scan mirroring :func:`_common_prefix_len`: bounded
    windows from each string's tail, then a char-by-char pinpoint on divergence.
    """
    na = len(a)
    nb = len(b)
    i = 0
    while i < hi:
        end = min(i + _SCAN_BLOCK, hi)
        if a[na - end : na - i] == b[nb - end : nb - i]:
            i = end
            continue
        while i < end and a[na - 1 - i] == b[nb - 1 - i]:
            i += 1
        return i
    return hi


def infer_edit_range(old: str, new: str) -> EditRange | None:
    """Infer the changed span between two full-source revisions.

    Returns ``None`` when the sources are identical.  Otherwise returns the
    minimal single span: the common prefix and common suffix are stripped and
    what remains is the replaced region.
    """
    if old is new or old == new:
        return None
    n_old = len(old)
    n_new = len(new)
    limit = n_old if n_old < n_new else n_new
    start = _common_prefix_len(old, new, limit)
    # Common suffix, not crossing back past the prefix in either string.
    suffix_max = min(n_old - start, n_new - start)
    slen = _common_suffix_len(old, new, suffix_max)
    return EditRange(start=start, old_end=n_old - slen, new_end=n_new - slen)


def _shift_command(cmd: SegmentedCommand, offset_delta: int, line_delta: int) -> SegmentedCommand:
    """Return *cmd* with all token/range positions shifted.

    Only valid for commands on lines strictly below the edit's last line, so
    that token columns are unchanged and shifting offset + line suffices.
    """
    return SegmentedCommand(
        range=shift_range(cmd.range, offset_delta, line_delta),
        argv=[shift_token(t, offset_delta, line_delta) for t in cmd.argv],
        texts=cmd.texts,
        single_token_word=cmd.single_token_word,
        all_tokens=[shift_token(t, offset_delta, line_delta) for t in cmd.all_tokens],
        preceding_comment=cmd.preceding_comment,
        is_partial=cmd.is_partial,
        partial_delimiter=cmd.partial_delimiter,
        expand_word=cmd.expand_word,
        subcommand=cmd.subcommand,
    )


def _shift_chunk(
    chunk: TopLevelChunk,
    *,
    index: int,
    offset_delta: int,
    line_delta: int,
) -> TopLevelChunk:
    return TopLevelChunk(
        index=index,
        start_offset=chunk.start_offset + offset_delta,
        end_offset=chunk.end_offset + offset_delta,
        source_hash=chunk.source_hash,  # text unchanged → hash unchanged
        commands=tuple(_shift_command(c, offset_delta, line_delta) for c in chunk.commands),
    )


def _chunks_equal_ignoring_index(a: TopLevelChunk, b: TopLevelChunk) -> bool:
    return (
        a.start_offset == b.start_offset
        and a.end_offset == b.end_offset
        and a.source_hash == b.source_hash
        and a.commands == b.commands
    )


def incremental_top_level_chunks(
    old_source: str,
    old_chunks: Sequence[TopLevelChunk],
    new_source: str,
    edit: EditRange,
) -> list[TopLevelChunk] | None:
    """Rebuild the top-level chunk list for *new_source* incrementally.

    Reuses the prefix verbatim and the post-edit suffix offset-shifted, and
    re-segments only the window straddling the edit (the middle plus the first
    post-edit command, which serves as a *validation* command).  The shifted
    suffix tail is reused only when that freshly re-segmented validation
    command matches the shifted old one — proving the lexer reaches the same
    clean state at the reuse boundary, so an edit that unbalances a delimiter
    and bleeds into the suffix is caught and the whole result rejected.

    Returns ``None`` when the fast path cannot guarantee a byte-identical
    result (the caller then re-segments fully): no clean multi-chunk suffix
    below the edit's last line, a partial/recovered command in a reused
    region, or a failed boundary validation.
    """
    if not old_chunks:
        return None
    n = len(old_chunks)

    start = edit.start
    old_end = edit.old_end
    offset_delta = edit.offset_delta
    line_delta = new_source[start : edit.new_end].count("\n") - old_source[start:old_end].count(
        "\n"
    )

    # Prefix: chunks whose tile ends strictly before the edit start are
    # untouched.  The strict ``<`` matters: a chunk whose tile boundary
    # *equals* ``start`` (an insertion exactly at the next command's start)
    # has its ``end_offset`` — the next command's start — pushed by the edit,
    # so it must be re-tiled in the window rather than reused verbatim.
    prefix_count = 0
    for chunk in old_chunks:
        if chunk.end_offset < start:
            prefix_count += 1
        else:
            break
    prefix = list(old_chunks[:prefix_count])
    if any(cmd.is_partial for chunk in prefix for cmd in chunk.commands):
        return None

    # Suffix: chunks beginning after the first newline at/after old_end sit on
    # lines strictly below the edit, so offset+line shifting (columns
    # unchanged) is exact.  We need at least one reusable tail chunk *after*
    # the validation chunk, so require two such chunks.
    nl = old_source.find("\n", old_end)
    if nl == -1:
        return None  # edit's last line runs to EOF — no clean suffix.
    suffix_min = nl + 1
    val_idx = None  # index of the validation chunk (first reusable post-edit chunk)
    for idx in range(prefix_count, n):
        if old_chunks[idx].start_offset >= suffix_min:
            val_idx = idx
            break
    if val_idx is None or val_idx >= n - 1:
        return None  # need a validation chunk and at least one tail chunk to reuse.
    reuse_old = old_chunks[val_idx + 1 :]
    if any(cmd.is_partial for chunk in old_chunks[val_idx:] for cmd in chunk.commands):
        return None

    # Window: covers the middle plus the validation chunk.  The first command
    # in the window is the one at ``p`` (the first non-prefix command); the
    # window text begins at ``ws`` — the previous command's end — so that any
    # comment lines in the gap before ``p`` are seen and attached to the
    # command at ``p`` exactly as a full segmentation would (comments attach
    # forward but physically sit in the previous chunk's tile region).
    p = 0 if prefix_count == 0 else old_chunks[prefix_count].start_offset
    # +1: range.end.offset is the command's last character (inclusive), so the
    # gap before p begins one byte past it.
    ws = 0 if prefix_count == 0 else old_chunks[prefix_count - 1].commands[-1].range.end.offset + 1
    val_end_old = old_chunks[val_idx].end_offset  # == old_chunks[val_idx+1].start_offset
    if ws > p or p > old_chunks[val_idx].start_offset:
        return None
    val_end_new = val_end_old + offset_delta
    window_text = new_source[ws:val_end_new]

    buf = DocumentBuffer.from_source(new_source)
    pos_ws = buf.offset_to_position(ws)
    pos_end = buf.offset_to_position(val_end_new)
    # Anchored at absolute offset ws with recovery disabled (a body_token
    # slice); ws sits just past a command in the unchanged prefix, so the
    # lexer's entry state there is clean.
    window_cmds = segment_commands(
        window_text,
        body_token=Token(
            type=TokenType.ESC,
            text=window_text,
            start=pos_ws,
            end=pos_end,
        ),
    )
    # The gap [ws, p) must contain only separators/comments; the first real
    # command must be the one at p, or our boundary assumptions are wrong.
    if not window_cmds or window_cmds[0].range.start.offset != p:
        return None
    window_chunks = tile_commands(
        window_cmds,
        new_source,
        start_index=prefix_count,
        final_end=val_end_new,
    )
    if not window_chunks:
        return None

    # The window's last chunk is the validation command: it must equal the old
    # first post-edit chunk shifted into place, or the edit bled across the
    # boundary and the suffix tail cannot be reused.
    expected_val = _shift_chunk(
        old_chunks[val_idx],
        index=window_chunks[-1].index,
        offset_delta=offset_delta,
        line_delta=line_delta,
    )
    if not _chunks_equal_ignoring_index(window_chunks[-1], expected_val):
        return None

    # Reuse the shifted suffix tail after the validated boundary.
    tail_start_index = prefix_count + len(window_chunks)
    tail_chunks = [
        _shift_chunk(
            chunk, index=tail_start_index + i, offset_delta=offset_delta, line_delta=line_delta
        )
        for i, chunk in enumerate(reuse_old)
    ]

    return prefix + window_chunks + tail_chunks

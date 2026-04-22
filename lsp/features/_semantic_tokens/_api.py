from __future__ import annotations

import logging
import time
from bisect import bisect_right

from core.analysis.semantic_model import AnalysisResult

from ._bigip import (
    _collect_apl_tokens,
    _collect_bigip_embedded_irules_object_tokens,
    _collect_bigip_tokens,
    _collect_embedded_tcl_tokens,
    _collect_irules_object_tokens,
)
from ._collect import _collect_tokens

log = logging.getLogger(__name__)


def semantic_tokens_full(
    source: str,
    analysis: AnalysisResult | None = None,
    *,
    is_bigip_conf: bool = False,
    is_irules: bool = False,
    is_apl: bool = False,
    chunk_token_cache: list[list[tuple[int, int, int, int, int]] | None] | None = None,
    chunk_line_ranges: list[tuple[int, int, int, int]] | None = None,
    line_starts: list[int] | tuple[int, ...] | None = None,
) -> list[int]:
    """Produce the flat list of 5-int encoded semantic tokens for the source.

    When *analysis* is provided, regex variable positions identified by the
    analyser are highlighted as ``regexp`` tokens instead of their normal type.

    When *chunk_token_cache* and *chunk_line_ranges* are provided, cached
    absolute tokens are reused for chunks that have them, and only dirty
    chunks are extracted from a full token computation.  The cache entries
    are updated in-place so that future calls benefit from the cache.
    """
    t0 = time.perf_counter()

    # Check if we have a full cache hit (before building regex_positions).
    if chunk_token_cache is not None and chunk_line_ranges is not None:
        if all(entry is not None for entry in chunk_token_cache):
            # Full cache hit — assemble from cached absolute tokens.
            raw_tokens: list[tuple[int, int, int, int, int]] = []
            for entry in chunk_token_cache:
                assert entry is not None
                raw_tokens.extend(entry)
            result = _delta_encode(raw_tokens)
            log.info(
                "[timing] semantic_tokens_full %.0fms (full cache hit, tokens=%d)",
                (time.perf_counter() - t0) * 1000,
                len(result) // 5,
            )
            return result

    regex_positions: frozenset[tuple[int, int]] = frozenset()
    if analysis is not None:
        regex_positions = analysis.regex_position_set

    # Collect all tokens with absolute positions
    raw_tokens: list[tuple[int, int, int, int, int]] = []
    base_tokens: list[tuple[int, int, int, int, int]] = []
    t_collect = time.perf_counter()
    _collect_tokens(base_tokens, source, regex_positions=regex_positions, _line_starts=line_starts)
    t_after_collect = time.perf_counter()
    if is_bigip_conf:
        # Run full Tcl tokenisation on embedded iRule/iApp bodies.
        embedded_tokens: list[tuple[int, int, int, int, int]] = []
        body_ranges = _collect_embedded_tcl_tokens(
            embedded_tokens, source, regex_positions=regex_positions
        )
        if body_ranges:
            # Remove tokens from the whole-file Tcl pass that overlap
            # with embedded body ranges; the body-specific tokens are
            # richer.  BIG-IP overlay tokens are added separately.
            # Use bisect for O(T log R) instead of O(T × R).
            body_ranges.sort()
            _range_starts = [s for s, _e in body_ranges]
            _range_ends = [e for _s, e in body_ranges]

            def _in_body(line: int) -> bool:
                idx = bisect_right(_range_starts, line) - 1
                return idx >= 0 and line <= _range_ends[idx]

            base_tokens = [tok for tok in base_tokens if not _in_body(tok[0])]
        raw_tokens.extend(base_tokens)
        raw_tokens.extend(embedded_tokens)
        _collect_bigip_tokens(raw_tokens, source)
        _collect_bigip_embedded_irules_object_tokens(raw_tokens, source)
    else:
        raw_tokens.extend(base_tokens)
    if is_irules:
        _collect_irules_object_tokens(raw_tokens, source)
    if is_apl:
        _collect_apl_tokens(raw_tokens, source)

    # Sort by position (line, then character) for correct delta encoding.
    raw_tokens.sort(key=lambda t: (t[0], t[1]))

    # Populate chunk cache if provided.
    # Use (line, col) boundaries so chunks sharing a line get non-overlapping
    # token sets (e.g. semicolon-separated commands on the same line).
    # Binary search (bisect) gives O(chunks * log(tokens)) instead of
    # O(chunks * tokens) — significant for large files with many chunks.
    if chunk_token_cache is not None and chunk_line_ranges is not None:
        from bisect import bisect_left

        keys = [(t[0], t[1]) for t in raw_tokens]
        for i, (sl, sc, el, ec) in enumerate(chunk_line_ranges):
            if i < len(chunk_token_cache) and chunk_token_cache[i] is None:
                lo = bisect_left(keys, (sl, sc))
                hi = bisect_left(keys, (el, ec))
                chunk_token_cache[i] = raw_tokens[lo:hi]

    result = _delta_encode(raw_tokens)
    t_end = time.perf_counter()
    log.info(
        "[timing] semantic_tokens_full %.0fms (collect=%.0fms, encode=%.0fms, tokens=%d, lines=%d)",
        (t_end - t0) * 1000,
        (t_after_collect - t_collect) * 1000,
        (t_end - t_after_collect) * 1000,
        len(result) // 5,
        source.count("\n") + 1,
    )
    return result


def precompute_chunk_tokens(
    source: str,
    chunk_line_ranges: list[tuple[int, int, int, int]],
    analysis: AnalysisResult | None = None,
    *,
    is_bigip_conf: bool = False,
    is_irules: bool = False,
    is_apl: bool = False,
) -> list[list[tuple[int, int, int, int, int]]]:
    """Pre-compute per-chunk semantic tokens in the background thread.

    Runs the same ``_collect_tokens`` pass as ``semantic_tokens_full``
    but partitions the result into per-chunk absolute-position token
    lists suitable for storing in ``ChunkCache.semantic_tokens_abs``.

    This eliminates the redundant full-document lex that would otherwise
    happen when the editor requests semantic tokens before the background
    analysis finishes, or on the ``workspace/semanticTokens/refresh``
    that follows the analysis pass.
    """
    from bisect import bisect_left, bisect_right

    t0 = time.perf_counter()

    regex_positions: frozenset[tuple[int, int]] = frozenset()
    if analysis is not None:
        regex_positions = analysis.regex_position_set

    raw_tokens: list[tuple[int, int, int, int, int]] = []
    base_tokens: list[tuple[int, int, int, int, int]] = []
    _collect_tokens(base_tokens, source, regex_positions=regex_positions)

    if is_bigip_conf:
        embedded_tokens: list[tuple[int, int, int, int, int]] = []
        body_ranges = _collect_embedded_tcl_tokens(
            embedded_tokens, source, regex_positions=regex_positions
        )
        if body_ranges:
            body_ranges.sort()
            _range_starts = [s for s, _e in body_ranges]
            _range_ends = [e for _s, e in body_ranges]

            def _in_body(line: int) -> bool:
                idx = bisect_right(_range_starts, line) - 1
                return idx >= 0 and line <= _range_ends[idx]

            base_tokens = [tok for tok in base_tokens if not _in_body(tok[0])]
        raw_tokens.extend(base_tokens)
        raw_tokens.extend(embedded_tokens)
        _collect_bigip_tokens(raw_tokens, source)
        _collect_bigip_embedded_irules_object_tokens(raw_tokens, source)
    else:
        raw_tokens.extend(base_tokens)
    if is_irules:
        _collect_irules_object_tokens(raw_tokens, source)
    if is_apl:
        _collect_apl_tokens(raw_tokens, source)

    raw_tokens.sort(key=lambda t: (t[0], t[1]))

    # Partition into per-chunk lists using binary search.
    keys = [(t[0], t[1]) for t in raw_tokens]
    chunk_tokens: list[list[tuple[int, int, int, int, int]]] = []
    for sl, sc, el, ec in chunk_line_ranges:
        lo = bisect_left(keys, (sl, sc))
        hi = bisect_left(keys, (el, ec))
        chunk_tokens.append(raw_tokens[lo:hi])

    log.info(
        "[timing] precompute_chunk_tokens %.0fms (tokens=%d, chunks=%d)",
        (time.perf_counter() - t0) * 1000,
        len(raw_tokens),
        len(chunk_line_ranges),
    )
    return chunk_tokens


def _delta_encode(raw_tokens: list[tuple[int, int, int, int, int]]) -> list[int]:
    """Convert absolute-position tokens to LSP delta-encoded format.

    *raw_tokens* must already be sorted by ``(line, char)``.
    Pre-allocates the output list to avoid per-token temporary allocations.
    """
    n = len(raw_tokens)
    data = [0] * (n * 5)
    prev_line = 0
    prev_char = 0
    idx = 0

    for line, char, length, type_idx, modifiers in raw_tokens:
        delta_line = line - prev_line
        delta_char = char - prev_char if delta_line == 0 else char
        data[idx] = delta_line
        data[idx + 1] = delta_char
        data[idx + 2] = length
        data[idx + 3] = type_idx
        data[idx + 4] = modifiers
        idx += 5
        prev_line = line
        prev_char = char

    return data


def compute_semantic_tokens_edits(
    old_data: list[int],
    new_data: list[int],
) -> list[tuple[int, int, list[int]]]:
    """Compute a single spanning edit to transform *old_data* into *new_data*.

    Returns a list of ``(start, delete_count, insert_data)`` tuples
    suitable for ``SemanticTokensEdit``.  Operates on the flat 5-int
    encoded arrays.

    The algorithm finds the longest common prefix and suffix, then
    emits a single edit for the differing middle section.  This is
    not truly minimal for multiple disjoint changes (it covers the
    entire range from the first to the last difference), but is
    optimal for the common case of a single contiguous change region
    (which is what single-line or multi-line edits produce).
    """
    old_len = len(old_data)
    new_len = len(new_data)

    # Find common prefix length (must be 5-int aligned).
    min_len = min(old_len, new_len)
    prefix = 0
    while prefix < min_len and old_data[prefix] == new_data[prefix]:
        prefix += 1
    # Align to 5-int token boundary.
    prefix = (prefix // 5) * 5

    # Find common suffix length (must be 5-int aligned).
    suffix = 0
    while (
        suffix < (min_len - prefix)
        and old_data[old_len - 1 - suffix] == new_data[new_len - 1 - suffix]
    ):
        suffix += 1
    suffix = (suffix // 5) * 5

    delete_count = old_len - prefix - suffix
    insert_data = new_data[prefix : new_len - suffix]

    if delete_count == 0 and len(insert_data) == 0:
        return []  # identical

    return [(prefix, delete_count, insert_data)]

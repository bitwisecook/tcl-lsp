"""Property fuzz for the per-chunk semantic-token cache's soundness invariant.

The incremental server caches semantic tokens *per chunk* and, on each edit,
reuses the cache for the clean prefix that ``find_first_dirty_chunk`` reports
unchanged.  Soundness therefore rests on one invariant:

    If a chunk lands in the reused clean prefix (its dirty-key is unchanged),
    its precomputed semantic tokens must be byte-identical.

A violation means the editor would render stale tokens after an edit — exactly
the class of bug behind the chunk-cache fixes (a trailing-content hash gap, an
unclosed delimiter swallowing whitespace into a token, and an equal-length
newline swap shifting a chunk's line without moving its offset).  This fuzzer
hammers that invariant with random sources and random edits — including hostile,
delimiter-heavy, recovery-triggering shapes — so a regression in either the chunk
hash (``_chunk_content_end``) or the dirty-key (``find_first_dirty_chunk``)
surfaces here deterministically.
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.command_segmenter import (
    find_first_dirty_chunk,
    segment_top_level_chunks,
)
from server.features._semantic_tokens._api import precompute_chunk_tokens
from server.workspace.document_state import _chunk_line_range
from shared.document_buffer import DocumentBuffer

# A token-bearing alphabet skewed toward the things that make chunking hard:
# every delimiter, separators (`;`, newline, space), comments, and a few real
# commands so error-recovery fires and chunk boundaries actually move.
_ATOMS = (
    'set proc if foreach while {} [] $x "q" ; \n# c\nputs expr namespace eval '
    '{ } [ ] " $ ; \n x a 1'
)


def _rand_source(rng: random.Random) -> str:
    return "".join(rng.choice(_ATOMS) for _ in range(rng.randint(0, 140)))


def _rand_edit(rng: random.Random, s: str) -> str:
    if not s:
        return rng.choice(_ATOMS)
    i = rng.randint(0, len(s))
    kind = rng.randint(0, 3)
    if kind == 0:  # insert
        ins = "".join(rng.choice(_ATOMS) for _ in range(rng.randint(1, 4)))
        return s[:i] + ins + s[i:]
    if kind == 1:  # delete a span
        return s[:i] + s[min(len(s), i + rng.randint(1, 5)) :]
    if kind == 2:  # equal-length replace (exercises offset-stable line shifts)
        j = min(len(s), i + rng.randint(1, 5))
        rep = "".join(rng.choice('set [ ] { } $ " x ;\n') for _ in range(j - i))
        return s[:i] + rep + s[j:]
    # targeted whitespace<->newline flip in place (offset-preserving line shift)
    j = rng.randint(0, max(0, len(s) - 1))
    if not s:
        return s
    flip = "\n" if s[j] != "\n" else " "
    return s[:j] + flip + s[j + 1 :]


def _keyed_chunk_tokens(src: str):
    """Return (chunks, per-chunk token lists) for *src*."""
    chunks = segment_top_level_chunks(src)
    buf = DocumentBuffer.from_source(src)
    ranges = [_chunk_line_range(buf, c) for c in chunks]
    return chunks, precompute_chunk_tokens(src, ranges)


class TestChunkCacheSoundnessFuzz:
    @pytest.mark.parametrize("seed", [0xC0FFEE, 0x5EED, 0xBADF00D, 2024, 99])
    def test_clean_prefix_chunks_have_identical_tokens(self, seed):
        rng = random.Random(seed)
        clean_prefixes_checked = 0
        for _ in range(8000):
            a = _rand_source(rng)
            b = _rand_edit(rng, a)
            ca, ta = _keyed_chunk_tokens(a)
            cb, tb = _keyed_chunk_tokens(b)
            # The reused region is exactly the clean prefix find_first_dirty_chunk
            # reports — model it precisely.
            dirty = find_first_dirty_chunk(ca, cb)
            for i in range(min(dirty, len(ca), len(cb))):
                clean_prefixes_checked += 1
                assert ta[i] == tb[i], (
                    "clean-prefix chunk reused with different semantic tokens:\n"
                    f"  a={a!r}\n  b={b!r}\n  chunk {i}: {ta[i]} != {tb[i]}"
                )
        # Guard against the test silently never exercising reuse.
        assert clean_prefixes_checked > 0


class TestIncrementalEqualsFreshFuzz:
    """The incremental chunker must stay byte-identical to a from-scratch
    segmentation — including the token-aware chunk hash — over hostile edits."""

    @pytest.mark.parametrize("seed", [1, 7, 256, 1024, 0x5EED])
    def test_incremental_chunks_equal_fresh(self, seed):
        from compiler.parsing.incremental import (
            incremental_top_level_chunks,
            infer_edit_range,
        )

        rng = random.Random(seed)
        fired = 0
        for _ in range(6000):
            old = _rand_source(rng)
            new = _rand_edit(rng, old)
            edit = infer_edit_range(old, new)
            if edit is None:
                continue
            old_chunks = segment_top_level_chunks(old)
            inc = incremental_top_level_chunks(old, old_chunks, new, edit)
            if inc is None:
                continue
            fired += 1
            fresh = segment_top_level_chunks(new)
            assert [(c.start_offset, c.end_offset, c.source_hash) for c in inc] == [
                (c.start_offset, c.end_offset, c.source_hash) for c in fresh
            ], f"incremental != fresh:\n  old={old!r}\n  new={new!r}"
        assert fired > 0

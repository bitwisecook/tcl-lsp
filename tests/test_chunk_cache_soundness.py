# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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


# Recovery-biased corpus.  The character-soup generator above almost never
# produces the one shape that drives error recovery — an unclosed delimiter
# followed, lines later, by a *known command* the segmenter resumes on — so it
# leaves the partial-command / over-running-token paths (the hard part) largely
# untested.  This generator interleaves unclosed openers with real commands so
# recovery fires in the majority of sources (asserted by a coverage guard).
_KNOWN_CMDS = [
    "set a 1",
    "proc p {} {}",
    "puts hi",
    "if {1} {}",
    "return",
    "expr {1 + 2}",
    "namespace eval n {}",
    "foreach x $l {}",
    "while {1} {}",
]
_OPENERS = ["{", "[", '"', "set x {", "puts [", 'set y "', "if {", "proc q {"]
_NOISE = ["}", "]", '"', "# comment here", "x", "$v", ";", "  ", "a b c"]


def _rand_recovery_source(rng: random.Random) -> str:
    parts = []
    for _ in range(rng.randint(2, 12)):
        r = rng.random()
        parts.append(
            rng.choice(_OPENERS)
            if r < 0.4
            else rng.choice(_KNOWN_CMDS)
            if r < 0.8
            else rng.choice(_NOISE)
        )
    return "\n".join(parts) + "\n"


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
    @pytest.mark.parametrize("seed", [0xC0FFEE, 0x5EED, 0xBADF00D])
    def test_clean_prefix_chunks_have_identical_tokens(self, seed):
        rng = random.Random(seed)
        clean_prefixes_checked = 0
        for _ in range(4000):
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

    @pytest.mark.parametrize("seed", [1, 256, 0x5EED])
    def test_incremental_chunks_equal_fresh(self, seed):
        from compiler.parsing.incremental import (
            incremental_top_level_chunks,
            infer_edit_range,
        )

        rng = random.Random(seed)
        fired = 0
        for _ in range(4000):
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


class TestRecoveryCacheSoundnessFuzz:
    """The same soundness invariant, but on a corpus where error *recovery*
    fires in most sources — the hard case, exercising partial commands, the
    over-running EOF token that signals them, and the recovered-command chunks
    split off after the broken one."""

    @pytest.mark.parametrize("seed", [12345, 0xC0FFEE])
    def test_clean_prefix_chunks_have_identical_tokens(self, seed):
        rng = random.Random(seed)
        sources_with_recovery = 0
        sources = 0
        clean_prefixes_checked = 0
        for _ in range(4000):
            a = _rand_recovery_source(rng)
            b = _rand_edit(rng, a)
            ca, ta = _keyed_chunk_tokens(a)
            cb, tb = _keyed_chunk_tokens(b)
            sources += 1
            if any(c.commands[0].is_partial for c in ca):
                sources_with_recovery += 1
            dirty = find_first_dirty_chunk(ca, cb)
            for i in range(min(dirty, len(ca), len(cb))):
                clean_prefixes_checked += 1
                assert ta[i] == tb[i], (
                    "clean-prefix chunk reused with different semantic tokens:\n"
                    f"  a={a!r}\n  b={b!r}\n  chunk {i}: {ta[i]} != {tb[i]}"
                )
        # Coverage guard: this corpus must actually drive recovery, or it is not
        # testing the path it exists for.
        assert sources_with_recovery > sources // 4
        assert clean_prefixes_checked > 0

    @pytest.mark.parametrize("seed", [12345, 0xC0FFEE])
    def test_incremental_equals_fresh_under_recovery(self, seed):
        from compiler.parsing.incremental import (
            incremental_top_level_chunks,
            infer_edit_range,
        )

        rng = random.Random(seed)
        fired = 0
        for _ in range(4000):
            old = _rand_recovery_source(rng)
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

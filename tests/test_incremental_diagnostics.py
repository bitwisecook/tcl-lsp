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

"""Tests for the per-proc diagnostic memoization primitives (Phase 6 foundation)."""

from __future__ import annotations

from lsprotocol import types

from server.features.incremental_diagnostics import (
    SHIMMER_CODES,
    ProcDiagEntry,
    ProcInfo,
    memoize_body_local,
    partition_by_proc,
    reoffset_diagnostics,
)


def _diag(line: int, char: int, code: str, end_line: int | None = None) -> types.Diagnostic:
    el = end_line if end_line is not None else line
    return types.Diagnostic(
        range=types.Range(
            start=types.Position(line=line, character=char),
            end=types.Position(line=el, character=char + 3),
        ),
        message=f"msg-{code}",
        code=code,
    )


class TestPartition:
    def test_attributes_to_owning_proc(self):
        # proc A: lines 0-3, proc B: lines 5-8, top-level: line 10
        diags = [_diag(1, 4, "W220"), _diag(6, 4, "W210"), _diag(10, 0, "W123")]
        spans = [("::a", 0, 3), ("::b", 5, 8)]
        per_proc, rest = partition_by_proc(diags, spans)
        assert [d.code for d in per_proc["::a"]] == ["W220"]
        assert [d.code for d in per_proc["::b"]] == ["W210"]
        assert [d.code for d in rest] == ["W123"]

    def test_empty_procs_have_empty_buckets(self):
        diags = [_diag(10, 0, "W123")]
        per_proc, rest = partition_by_proc(diags, [("::a", 0, 3)])
        assert per_proc["::a"] == []
        assert [d.code for d in rest] == ["W123"]


class TestReoffset:
    def test_shift_down_by_line_delta(self):
        # proc was at line 5, cached diag at line 7 (2 lines into proc).
        entry = ProcDiagEntry(
            start_line=5,
            start_char=0,
            diagnostics=(_diag(7, 8, "W220"),),
        )
        # proc now at line 12 (edit above added 7 lines).
        out = reoffset_diagnostics(entry, new_start_line=12)
        assert out[0].range.start.line == 14  # 7 + (12 - 5)
        assert out[0].range.start.character == 8  # unchanged
        assert out[0].range.end.line == 14

    def test_shift_up(self):
        entry = ProcDiagEntry(
            start_line=20,
            start_char=0,
            diagnostics=(_diag(25, 4, "W210", end_line=26),),
        )
        out = reoffset_diagnostics(entry, new_start_line=10)
        assert out[0].range.start.line == 15  # 25 - 10
        assert out[0].range.end.line == 16  # 26 - 10
        assert out[0].range.start.character == 4

    def test_no_move_returns_equivalent(self):
        d = _diag(7, 8, "W220")
        entry = ProcDiagEntry(start_line=5, start_char=0, diagnostics=(d,))
        out = reoffset_diagnostics(entry, new_start_line=5)
        assert out[0].range.start.line == 7
        assert out[0].code == "W220"

    def test_related_information_shifted(self):
        rel = types.DiagnosticRelatedInformation(
            location=types.Location(
                uri="file:///x.tcl",
                range=types.Range(
                    start=types.Position(line=8, character=2),
                    end=types.Position(line=8, character=5),
                ),
            ),
            message="related",
        )
        d = types.Diagnostic(
            range=types.Range(
                start=types.Position(line=7, character=0),
                end=types.Position(line=7, character=3),
            ),
            message="m",
            code="W220",
            related_information=[rel],
        )
        entry = ProcDiagEntry(start_line=5, start_char=0, diagnostics=(d,))
        out = reoffset_diagnostics(entry, new_start_line=15)
        assert out[0].range.start.line == 17  # 7 + 10
        assert out[0].related_information is not None
        assert out[0].related_information[0].location.range.start.line == 18  # 8 + 10

    def test_reoffset_equals_recompute_property(self):
        # Re-offsetting a cached entry by Δ must equal the diagnostic the same
        # (byte-identical) proc would produce at the new position — modelled
        # here as: shifting by Δ twice via two paths agrees.
        base = [_diag(3, 4, "W220"), _diag(5, 8, "W210", end_line=6)]
        entry = ProcDiagEntry(start_line=2, start_char=0, diagnostics=tuple(base))
        for new_line in (2, 9, 100, 0):
            out = reoffset_diagnostics(entry, new_start_line=new_line)
            d = new_line - 2
            assert [(x.range.start.line, x.range.start.character) for x in out] == [
                (3 + d, 4),
                (5 + d, 8),
            ]


class TestMemoizeBodyLocalShimmer:
    """End-to-end: memoized shimmer (reuse clean procs + re-offset, recompute
    dirty) must equal a full recompute byte-for-byte — the soundness gate for
    the per-proc diagnostic cache, on a real pass (shimmer is body-local)."""

    @staticmethod
    def _proc_infos(src, cu):
        out = []
        for q, p in cu.ir_module.procedures.items():
            body = src[p.range.start.offset : p.range.end.offset + 1]
            out.append(
                ProcInfo(
                    qname=q,
                    body_hash=hash(body),
                    start_line=p.range.start.line,
                    start_char=p.range.start.character,
                    end_line=p.range.end.line,
                )
            )
        return out

    @staticmethod
    def _full_shimmer(src, cu):
        from server.features.diagnostics import get_deep_diagnostics

        return [d for d in get_deep_diagnostics(src, {}, cu=cu) if d.code in SHIMMER_CODES]

    @staticmethod
    def _key(ds):
        return sorted(
            (d.code, d.range.start.line, d.range.start.character, d.range.end.line, d.message)
            for d in ds
        )

    def _run(self, src, cu, prev_cache):
        from server.features.diagnostics import get_deep_diagnostics

        return memoize_body_local(
            self._proc_infos(src, cu),
            prev_cache,
            lambda dirty: get_deep_diagnostics(src, {}, cu=cu, shimmer_target_procs=dirty),
            codes=SHIMMER_CODES,
        )

    def test_cold_matches_full(self):
        from compiler.compilation_unit import compile_source

        src = (
            'proc a {n} { set v 0 ; set v "t$n" ; set v [expr {$v+1}] ; return $v }\n'
            'proc b {n} { set w 0 ; set w "s$n" ; set w [expr {$w*2}] ; return $w }\n'
        )
        cu = compile_source(src)
        diags, _ = self._run(src, cu, {})
        assert self._key(diags) == self._key(self._full_shimmer(src, cu))

    def test_warm_edit_above_and_change_one_proc_equals_full(self):
        from compiler.compilation_unit import compile_source

        src1 = (
            'proc a {n} { set v 0 ; set v "t$n" ; set v [expr {$v+1}] ; return $v }\n'
            'proc b {n} { set w 0 ; set w "s$n" ; set w [expr {$w*2}] ; return $w }\n'
        )
        cu1 = compile_source(src1)
        _, cache = self._run(src1, cu1, {})

        # Insert 2 blank lines above (shifts both procs +2) AND change proc b's body.
        src2 = (
            "\n\n"
            'proc a {n} { set v 0 ; set v "t$n" ; set v [expr {$v+1}] ; return $v }\n'
            'proc b {n} { set w 0 ; set w "s$n" ; set w [expr {$w*2}] ; '
            'set w "again$w" ; return $w }\n'
        )
        cu2 = compile_source(src2)
        diags2, _ = self._run(src2, cu2, cache)
        # proc a reused + re-offset (+2 lines), proc b recomputed — byte-for-byte == full.
        assert self._key(diags2) == self._key(self._full_shimmer(src2, cu2))

    def test_pure_shift_reuses_without_recompute(self):
        from compiler.compilation_unit import compile_source

        src1 = 'proc a {n} { set v 0 ; set v "t$n" ; set v [expr {$v+1}] ; return $v }\n'
        cu1 = compile_source(src1)
        _, cache = self._run(src1, cu1, {})
        # Only shift (blank line above), no body change → all clean → reuse+re-offset.
        src2 = "\n" + src1
        cu2 = compile_source(src2)
        diags2, _ = self._run(src2, cu2, cache)
        assert self._key(diags2) == self._key(self._full_shimmer(src2, cu2))

    def test_top_level_shimmer_not_dropped(self):
        # Top-level (non-proc) shimmer lives in partition_by_proc's `rest`
        # bucket; the memoizer must still emit it, or it is silently lost.
        from compiler.compilation_unit import compile_source

        src = (
            'set t 0\nset t "v$t"\nset t [expr {$t+1}]\n'  # top-level shimmer (lines 0-2)
            'proc a {n} { set v 0 ; set v "t$n" ; set v [expr {$v+1}] ; return $v }\n'
        )
        cu = compile_source(src)
        full = self._full_shimmer(src, cu)
        # Guard: the fixture genuinely produces top-level shimmer (outside proc a).
        assert any(d.range.start.line < 3 for d in full), "fixture has no top-level shimmer"
        diags, _ = self._run(src, cu, {})  # cold
        assert self._key(diags) == self._key(full)

    def test_top_level_shimmer_survives_shift(self):
        from compiler.compilation_unit import compile_source

        src1 = (
            'set t 0\nset t "v$t"\nset t [expr {$t+1}]\n'
            'proc a {n} { set v 0 ; set v "t$n" ; set v [expr {$v+1}] ; return $v }\n'
        )
        cu1 = compile_source(src1)
        _, cache = self._run(src1, cu1, {})
        src2 = "\n" + src1  # shift everything down a line
        cu2 = compile_source(src2)
        diags2, _ = self._run(src2, cu2, cache)
        assert self._key(diags2) == self._key(self._full_shimmer(src2, cu2))


class TestMemoizedDeepEndToEnd:
    """The full deep-path orchestration as the live pipeline drives it:
    ``proc_diag_infos`` → ``split_clean_dirty`` → recompute shimmer only for the
    dirty procs → ``merge_memoized_deep``.  The merged set (every deep code, not
    just shimmer) must equal a full ``get_deep_diagnostics`` byte-for-byte.

    This complements ``TestMemoizeBodyLocalShimmer`` (which tests the leaf
    primitive) by gating the non-body-local passthrough + cache plumbing that
    ``test_incremental_update`` does not reach (it only sees analyser diags)."""

    @staticmethod
    def _full_deep(src, cu):
        from server.features.diagnostics import get_deep_diagnostics

        return get_deep_diagnostics(src, {}, cu=cu)

    @staticmethod
    def _memoized(src, cu, prev_cache):
        from server.features.diagnostics import get_deep_diagnostics
        from server.features.incremental_diagnostics import (
            merge_memoized_deep,
            proc_diag_infos,
            split_clean_dirty,
        )

        infos = proc_diag_infos(cu)
        assert infos is not None
        _clean, dirty = split_clean_dirty(infos, prev_cache)
        full = get_deep_diagnostics(src, {}, cu=cu, shimmer_target_procs=dirty)
        return merge_memoized_deep(infos, prev_cache, full)

    @staticmethod
    def _key(ds):
        return sorted(
            (d.code, d.range.start.line, d.range.start.character, d.range.end.line, d.message)
            for d in ds
        )

    _A = 'proc a {n} { set v 0 ; set v "t$n" ; set v [expr {$v+1}] ; return $v }\n'
    _B = 'proc b {n} { set w 0 ; set w "s$n" ; set w [expr {$w*2}] ; return $w }\n'

    def test_cold_equals_full(self):
        from compiler.compilation_unit import compile_source

        src = self._A + self._B
        cu = compile_source(src)
        merged, cache = self._memoized(src, cu, {})
        assert self._key(merged) == self._key(self._full_deep(src, cu))
        assert cache  # cold pass populates the per-proc cache

    def test_pure_shift_equals_full(self):
        from compiler.compilation_unit import compile_source

        src1 = self._A + self._B
        cu1 = compile_source(src1)
        _, cache = self._memoized(src1, cu1, {})
        src2 = "\n\n" + src1  # move both procs down 2 lines, bodies identical
        cu2 = compile_source(src2)
        merged2, _ = self._memoized(src2, cu2, cache)
        assert self._key(merged2) == self._key(self._full_deep(src2, cu2))

    def test_one_proc_changed_equals_full(self):
        from compiler.compilation_unit import compile_source

        src1 = self._A + self._B
        cu1 = compile_source(src1)
        _, cache = self._memoized(src1, cu1, {})
        # proc a unchanged (reused+re-offset), proc b body changed (recomputed).
        src2 = (
            "\n" + self._A + 'proc b {n} { set w 0 ; set w "s$n" ; set w [expr {$w*2}] ; '
            'set w "again$w" ; return $w }\n'
        )
        cu2 = compile_source(src2)
        merged2, _ = self._memoized(src2, cu2, cache)
        assert self._key(merged2) == self._key(self._full_deep(src2, cu2))

    def test_callee_edit_keeps_soundness(self):
        # An edit to one proc that other procs depend on (shared CFG context):
        # the context fingerprint folded into each proc's body_hash makes every
        # proc dirty, so the merged result still equals a full rebuild.
        from compiler.compilation_unit import compile_source

        src1 = (
            'proc callee {x} { upvar 1 $x ref ; set ref 0 ; set ref "v$ref" }\n'
            "proc caller {} { set local 1 ; callee local ; return $local }\n"
        )
        cu1 = compile_source(src1)
        _, cache = self._memoized(src1, cu1, {})
        src2 = (
            'proc callee {x} { upvar 1 $x ref ; set ref 0 ; set ref "v$ref" ; incr ref }\n'
            "proc caller {} { set local 1 ; callee local ; return $local }\n"
        )
        cu2 = compile_source(src2)
        merged2, _ = self._memoized(src2, cu2, cache)
        assert self._key(merged2) == self._key(self._full_deep(src2, cu2))

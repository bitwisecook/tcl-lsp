"""Per-proc diagnostic memoization primitives (Phase 6 foundation).

The expensive diagnostic passes re-run over the whole document on every edit
even though the per-proc compilation artefacts (``FunctionUnit``: cfg/ssa/
analysis) are already cached.  This module supplies the *leaf tier* of the
incremental query-DAG: cache the **diagnostics** a proc produces, keyed by the
same proc-cache key (body-hash) that already gates ``FunctionUnit`` reuse, and
**re-offset** them when an edit above merely shifts the (byte-identical) proc
to a new position.

Two pure operations, unit-tested in isolation so the rest of the wiring can
trust them:

* :func:`partition_by_proc` — assign each emitted diagnostic to the proc whose
  source line span contains it (or to the cross-proc / top-level bucket).
* :func:`reoffset_diagnostics` — shift a cached proc's diagnostics by the
  proc's ``Δline`` move.  Sound **only when the proc's body is byte-identical
  and its start character is unchanged** (a whole-line shift above keeps the
  internal column positions identical); the caller recomputes otherwise.

Soundness contract: the merged result (reused + recomputed) must be
byte-for-byte identical to a full rebuild — gated by
``tests/test_incremental_update.py``.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import TYPE_CHECKING

import attrs
from lsprotocol import types

if TYPE_CHECKING:
    from compiler.compilation_unit import CompilationUnit

# Diagnostic codes that are body-local (depend only on a proc's own
# FunctionUnit — cfg/ssa/types — which ``_proc_cache`` already treats as
# body-hash-keyed).  Safe to memoize per proc and re-offset on reuse.  Shimmer
# (S1xx) is fully body-local.  NOT here: cross-proc codes (W123/W307/W308,
# interproc-derived O-codes, taint-flow) and the optimiser (cross-document
# overlap resolution) — those always recompute.
SHIMMER_CODES = frozenset({"S100", "S101", "S102"})


@dataclass(frozen=True, slots=True)
class ProcDiagEntry:
    """Cached diagnostics for one proc, with the position they were computed at."""

    start_line: int
    start_char: int
    diagnostics: tuple[types.Diagnostic, ...]


@dataclass(frozen=True, slots=True)
class ProcInfo:
    """A proc's identity + position for one document version."""

    qname: str
    body_hash: int  # hash of the proc's source text (matches _proc_cache keying)
    start_line: int
    start_char: int
    end_line: int


def proc_diag_infos(cu: CompilationUnit) -> list[ProcInfo] | None:
    """Build the per-proc identity+position list for body-local memoization.

    The ``body_hash`` mirrors the ``FunctionUnit`` proc-cache validity key
    (``_build_proc_cache``) — proc source text + active stub overlay +
    CFG-context fingerprint + class-set fingerprint — but **excludes the proc's
    position**, so a proc that merely moved (byte-identical body, unchanged
    context) stays *clean* and its diagnostics are re-offset rather than
    recomputed.  Folding the stub + context + known-classes fingerprints into the
    hash keeps reuse sound: when a callee's upvar behaviour changes the caller's
    analysis (and thus its shimmer), or a class added elsewhere changes its
    object-type facts, the fingerprint changes and every proc becomes dirty —
    degrading to a full recompute exactly when it must.

    Returns ``None`` (caller falls back to a full, non-memoized pass) when the
    proc set cannot be positioned consistently — i.e. the ``FunctionUnit`` procs
    and the IR procs disagree — so we never under-emit by targeting a stale set.
    """
    from compiler.cfg import cfg_context_fingerprint, prepare_cfg_context
    from compiler.compilation_unit import compute_stub_fingerprint, known_classes_fingerprint

    ir_procs = cu.ir_module.procedures
    if set(ir_procs) != set(cu.procedures):
        return None

    stub_fingerprint = compute_stub_fingerprint(cu.source)
    upvar_procs, proc_params = prepare_cfg_context(cu.ir_module)
    context_fingerprint = cfg_context_fingerprint(upvar_procs, proc_params)
    known_classes_fp = known_classes_fingerprint(cu.known_classes)

    infos: list[ProcInfo] = []
    for qname, ir_proc in ir_procs.items():
        start = ir_proc.range.start
        # +1: range.end.offset is the proc's last character (inclusive).
        proc_src = cu.source[start.offset : ir_proc.range.end.offset + 1]
        body_hash = hash((proc_src, stub_fingerprint, context_fingerprint, known_classes_fp))
        infos.append(
            ProcInfo(
                qname=qname,
                body_hash=body_hash,
                start_line=start.line,
                start_char=start.character,
                end_line=ir_proc.range.end.line,
            )
        )
    return infos


def split_clean_dirty(
    cur: list[ProcInfo],
    prev_cache: dict[tuple[str, int], ProcDiagEntry],
) -> tuple[list[ProcInfo], frozenset[str]]:
    """Partition current procs into ``(clean, dirty_qnames)``.

    A proc is **clean** iff its ``(qname, body_hash)`` key is in *prev_cache*
    AND its start character is unchanged (the re-offset soundness condition);
    otherwise its qname is **dirty** and its body-local diagnostics must be
    recomputed.  This is the single source of truth used both to target the
    recompute (``shimmer_target_procs``) and to drive the merge.
    """
    clean: list[ProcInfo] = []
    dirty: set[str] = set()
    for p in cur:
        entry = prev_cache.get((p.qname, p.body_hash))
        if entry is not None and entry.start_char == p.start_char:
            clean.append(p)
        else:
            dirty.add(p.qname)
    return clean, frozenset(dirty)


def memoize_body_local(
    cur: list[ProcInfo],
    prev_cache: dict[tuple[str, int], ProcDiagEntry],
    recompute: Callable[[frozenset[str]], list[types.Diagnostic]],
    *,
    codes: frozenset[str],
) -> tuple[list[types.Diagnostic], dict[tuple[str, int], ProcDiagEntry]]:
    """Recompute only dirty procs' body-local diagnostics; reuse the rest.

    *cur* describes the current procs (qname, body-hash, position).  A proc is
    **clean** iff its ``(qname, body_hash)`` key is in *prev_cache* AND its
    start character is unchanged (re-offset soundness); otherwise **dirty**.

    *recompute(dirty)* runs the body-local pass(es) for the dirty set (plus
    top-level / anything outside a proc) and returns ALL their diagnostics; we
    keep only those whose ``code`` is in *codes* and whose line falls inside a
    dirty proc, partition them per proc into the new cache, and re-offset the
    cached entries for clean procs.  Returns ``(merged_body_local_diags,
    new_cache)`` — the caller concatenates these with the non-memoized
    (cross-proc / optimiser / other-code) diagnostics it computed separately.
    """
    dirty: frozenset[str]
    clean, dirty = split_clean_dirty(cur, prev_cache)

    recomputed = [d for d in recompute(dirty) if d.code in codes]

    # Partition recomputed diagnostics into dirty procs (by line span).  The
    # ``rest`` bucket holds diagnostics that fall in no dirty-proc span — i.e.
    # *top-level* (non-proc) diagnostics, which the recompute pass always emits
    # (a body-local pass still scans the top level).  Those are not memoized per
    # proc; they are always freshly recomputed, so they are emitted as-is.
    spans = [(p.qname, p.start_line, p.end_line) for p in cur if p.qname in dirty]
    per_proc, rest = partition_by_proc(recomputed, spans)

    new_cache: dict[tuple[str, int], ProcDiagEntry] = {}
    out: list[types.Diagnostic] = list(rest)

    for qname in dirty:
        p = next(pp for pp in cur if pp.qname == qname)
        diags = per_proc.get(qname, [])
        new_cache[(qname, p.body_hash)] = ProcDiagEntry(
            start_line=p.start_line, start_char=p.start_char, diagnostics=tuple(diags)
        )
        out.extend(diags)

    for p in clean:
        entry = prev_cache[(p.qname, p.body_hash)]
        new_cache[(p.qname, p.body_hash)] = entry  # carry forward
        out.extend(reoffset_diagnostics(entry, new_start_line=p.start_line))

    return out, new_cache


def merge_memoized_deep(
    cur: list[ProcInfo],
    prev_cache: dict[tuple[str, int], ProcDiagEntry],
    full_deep: list[types.Diagnostic],
    *,
    body_local_codes: frozenset[str] = SHIMMER_CODES,
) -> tuple[list[types.Diagnostic], dict[tuple[str, int], ProcDiagEntry]]:
    """Assemble the complete deep-diagnostic set incrementally from *full_deep*.

    *full_deep* is the output of a deep-diagnostics run whose body-local (e.g.
    shimmer) pass was restricted to the **dirty** procs of :func:`split_clean_dirty`
    (via ``get_deep_diagnostics(shimmer_target_procs=dirty)``).  So every
    non-body-local code (optimiser / taint / interproc / iRules-flow) is already
    complete and emitted as-is, while body-local codes are present only for dirty
    procs + the top level; clean procs' body-local diagnostics are reused
    (re-offset) from *prev_cache*.

    The caller must compute *full_deep*'s ``shimmer_target_procs`` from the
    **same** ``(cur, prev_cache)`` passed here, so the dirty set this function
    recomputes internally matches what was actually recomputed.  Returns
    ``(all_deep_diagnostics, new_cache)``; ``new_cache`` should be persisted for
    the next edit.
    """
    body_local_merged, new_cache = memoize_body_local(
        cur, prev_cache, lambda _dirty: full_deep, codes=body_local_codes
    )
    non_body_local = [d for d in full_deep if d.code not in body_local_codes]
    return non_body_local + body_local_merged, new_cache


def partition_by_proc(
    diagnostics: list[types.Diagnostic],
    spans: list[tuple[str, int, int]],
) -> tuple[dict[str, list[types.Diagnostic]], list[types.Diagnostic]]:
    """Split *diagnostics* into per-proc buckets + a cross-proc/top-level rest.

    *spans* are ``(qname, start_line, end_line)`` — non-overlapping (top-level
    procs don't nest).  A diagnostic belongs to the proc whose
    ``[start_line, end_line]`` contains its start line; everything else (top
    level, genuinely cross-proc) goes to the *rest* list.
    """
    ordered = sorted(spans, key=lambda s: s[1])
    per_proc: dict[str, list[types.Diagnostic]] = {q: [] for q, _, _ in ordered}
    rest: list[types.Diagnostic] = []
    for d in diagnostics:
        ln = d.range.start.line
        owner: str | None = None
        for q, lo, hi in ordered:
            if lo <= ln <= hi:
                owner = q
                break
            if lo > ln:
                break
        if owner is None:
            rest.append(d)
        else:
            per_proc[owner].append(d)
    return per_proc, rest


def _shift_range(r: types.Range, dline: int) -> types.Range:
    return types.Range(
        start=types.Position(line=r.start.line + dline, character=r.start.character),
        end=types.Position(line=r.end.line + dline, character=r.end.character),
    )


def _shift_location(loc: types.Location, dline: int) -> types.Location:
    return attrs.evolve(loc, range=_shift_range(loc.range, dline))


def reoffset_diagnostics(
    entry: ProcDiagEntry,
    new_start_line: int,
) -> list[types.Diagnostic]:
    """Shift a cached proc's diagnostics to its new line position.

    Only the line delta is applied: when an edit *above* a byte-identical proc
    moves it, every internal line shifts by the same ``Δline`` and column
    positions are unchanged.  The caller must only reuse an entry whose proc
    body is byte-identical AND whose start character is unchanged (see module
    docstring); under that contract this reconstruction is exact.

    ``related_information`` locations are shifted too (same-document case).
    """
    dline = new_start_line - entry.start_line
    if dline == 0:
        return list(entry.diagnostics)
    shifted: list[types.Diagnostic] = []
    for d in entry.diagnostics:
        related = d.related_information
        new_related = related
        if related:
            new_related = [
                attrs.evolve(ri, location=_shift_location(ri.location, dline)) for ri in related
            ]
        shifted.append(
            attrs.evolve(d, range=_shift_range(d.range, dline), related_information=new_related)
        )
    return shifted


__all__ = [
    "SHIMMER_CODES",
    "ProcDiagEntry",
    "ProcInfo",
    "memoize_body_local",
    "merge_memoized_deep",
    "partition_by_proc",
    "proc_diag_infos",
    "reoffset_diagnostics",
    "split_clean_dirty",
]

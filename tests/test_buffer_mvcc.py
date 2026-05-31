"""MVCC version registry on DocumentState.

A document keeps one *current* rope-backed buffer (the single buffer all
consumers read).  Edits produce new immutable versions that share structure with
their predecessors (the persistent treap rope).  The version registry holds each
materialised version **weakly**, so:

* an in-flight reader (request handler / analysis task) that captured an older
  version keeps it alive only while it holds it, and
* Python's GC reclaims any version no longer referenced — the registry never
  pins old versions itself.
"""

from __future__ import annotations

import gc
import sys
import time
import weakref
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.workspace.document_state import DocumentState
from shared.document_buffer import DocumentBuffer


def _materialise_versions(st: DocumentState) -> None:
    st.update_source_quick("set a 1\n", 1)
    _ = st.buffer  # materialise v1
    st.update_source_quick("set a 1\nset b 2\n", 2)
    _ = st.buffer  # materialise v2 (splices from v1's rope)
    st.update_source_quick("set a 1\nset b 2\nset c 3\n", 3)
    _ = st.buffer  # materialise v3 (current)


def test_current_version_is_registered_and_live():
    st = DocumentState(uri="file:///mvcc1.tcl")
    _materialise_versions(st)
    b3 = st.buffer_for_version(3)
    assert b3 is not None and b3.version == 3
    assert b3.source.endswith("set c 3\n")


def _pin_v2_then_advance(st: DocumentState) -> DocumentBuffer:
    """Materialise v1, materialise + return a strong ref to v2 (an in-flight
    reader's pin), then advance to current v3 — so v2 survives only via the
    returned ref."""
    st.update_source_quick("set a 1\n", 1)
    _ = st.buffer
    st.update_source_quick("set a 1\nset b 2\n", 2)
    pin = st.buffer  # in-flight reader pins v2 across the next edit
    st.update_source_quick("set a 1\nset b 2\nset c 3\n", 3)
    _ = st.buffer  # current v3
    return pin


def test_pinned_old_version_is_retrievable():
    st = DocumentState(uri="file:///mvcc2.tcl")
    pin = _pin_v2_then_advance(st)
    assert pin.version == 2
    # Retrievable while a reader still holds it, even though v3 is current.
    assert st.buffer_for_version(2) is pin


def test_unpinned_old_version_is_gc_reclaimed():
    st = DocumentState(uri="file:///mvcc3.tcl")
    pin = _pin_v2_then_advance(st)
    assert st.buffer_for_version(2) is not None  # present while pinned
    del pin
    gc.collect()
    # No snapshot holds v2 (current is v3), so once the reader drops it GC
    # reclaims it — the weak registry must now report it gone.
    assert st.buffer_for_version(2) is None


def test_current_version_survives_gc():
    st = DocumentState(uri="file:///mvcc4.tcl")
    _materialise_versions(st)
    gc.collect()
    # v3 is the current snapshot's buffer — a strong ref keeps it alive across
    # GC, and it must be the *right* buffer (correct version + content), not
    # merely some surviving object.
    b3 = st.buffer_for_version(3)
    assert b3 is not None and b3.version == 3
    assert b3.source.endswith("set c 3\n")
    # It is the very buffer the current snapshot exposes, not a stale revival.
    assert b3 is st.buffer


def test_registry_prunes_to_live_versions_only():
    st = DocumentState(uri="file:///mvcc5.tcl")
    _materialise_versions(st)
    gc.collect()
    live = dict(st._versions)
    # Only the current version remains resident (older quick-update versions are
    # not pinned by any snapshot and have been reclaimed).
    assert set(live) == {3}, f"expected only v3 live; got {sorted(live)}"


def test_unknown_version_returns_none():
    st = DocumentState(uri="file:///mvcc6.tcl")
    _materialise_versions(st)
    assert st.buffer_for_version(99) is None


# --- Stress: many edits on a large file must not leak versions --------------

_BIG_FILE = (
    Path(__file__).resolve().parent.parent
    / "tmp"
    / "tcllib-2.0"
    / "modules"
    / "fumagic"
    / "filetypes.tcl"
)


def _big_source(min_bytes: int) -> str:
    """A multi-megabyte Tcl source: the real filetypes.tcl when present, else a
    synthesised body of the same order of magnitude (CI without the corpus)."""
    if _BIG_FILE.exists():
        text = _BIG_FILE.read_text(errors="replace")
        if len(text) >= min_bytes:
            return text
    # Fallback: synthesise ~min_bytes of valid-ish Tcl.
    unit = 'set var_{i} "value {i}"\nputs $var_{i}\n'
    n = (min_bytes // len(unit.format(i=0))) + 1
    return "".join(unit.format(i=i) for i in range(n))


def _hammer_rope_versions(source: str, n_edits: int) -> tuple[int, float, str]:
    """Drive *n_edits* rope splices (``DocumentBuffer.from_edit``) over *source*,
    each a new immutable version recorded in a weak registry, holding a strong
    ref only to the *current* version (mirroring DocumentState: the live
    snapshot pins the current buffer, nothing pins superseded ones).

    Returns ``(peak_live_versions, elapsed_seconds, final_source)``.  ``peak``
    must stay O(1): the immutable treap shares structure across versions, and
    the weak registry lets GC reclaim every superseded version — so 20k
    versions of a multi-MB document never accumulate (which would be tens of GB
    and OOM under a strong-ref registry / leaking buffer cache).
    """
    from shared.rope import RopeEdit

    registry: weakref.WeakValueDictionary[int, DocumentBuffer] = weakref.WeakValueDictionary()
    buf = DocumentBuffer.from_source(source, 1)
    registry[1] = buf
    cur = source
    peak = len(dict(registry))
    span = max(1, len(cur) - 64)
    t0 = time.perf_counter()
    for k in range(2, n_edits + 2):
        pos = 32 + (k * 4099) % span
        ins = f"\ns{k} {k}\n"
        new = cur[:pos] + ins + cur[pos:]
        edit = RopeEdit(pos, pos, pos + len(ins), ins.count("\n"))
        # Reassigning `buf` drops the only strong ref to the prior version, so
        # the weak registry entry for it becomes collectable.
        buf = DocumentBuffer.from_edit(buf, new, edit, k)
        registry[k] = buf
        cur = new
        span = max(1, len(cur) - 64)
        if k % 1000 == 0:
            gc.collect()
            peak = max(peak, len(dict(registry)))
    elapsed = time.perf_counter() - t0
    gc.collect()
    peak = max(peak, len(dict(registry)))
    # `buf` (the current version) is still strongly held here, so it stays live.
    assert registry.get(n_edits + 1) is buf
    return peak, elapsed, cur


@pytest.mark.slow
def test_mvcc_no_version_leak_under_tens_of_thousands_of_edits():
    """Punish the MVCC: tens of thousands of rope-splice versions of a
    multi-megabyte file (filetypes.tcl, ~1.3 MB).

    Each edit forks a new immutable version sharing structure with its
    predecessor.  The weak registry + GC must keep the resident version count
    O(1): a strong-ref registry (or a buffer cache that never releases) would
    retain ~20k versions of a >1 MB document — tens of GB — and OOM.  A loose
    throughput bound also catches an O(n)-per-splice regression."""
    source = _big_source(1_000_000)
    assert len(source) >= 1_000_000, "stress test needs a multi-MB source"

    peak, elapsed, final = _hammer_rope_versions(source, 20_000)

    assert peak <= 4, f"version registry leaked old versions: peak_live={peak}"
    # The edited source grew by 20k small insertions but is still coherent.
    assert len(final) > len(source)
    # 20k O(log n) splices on a 1.3 MB rope should be well under a minute; the
    # bound flags an accidental O(n)-per-splice (or O(n²) total) regression.
    assert elapsed < 60.0, f"20k rope splices took {elapsed:.1f}s (splice no longer O(log n)?)"


def test_mvcc_no_version_leak_small_scale():
    """Fast (ci-fast) variant of the no-leak invariant: 2k rope-splice versions
    on a ~200 KB buffer, checked on every run."""
    source = _big_source(200_000)[:200_000]
    peak, _elapsed, _final = _hammer_rope_versions(source, 2_000)
    assert peak <= 4, f"version registry leaked: peak_live={peak}"


def test_document_state_quick_update_registers_and_reclaims_versions():
    """Integration: the real update_source_quick path registers each version and
    lets superseded ones GC (a smaller many-command source so incremental
    segmentation keeps the per-edit cost low)."""
    base = "".join(f"proc p{i} {{}} {{ return {i} }}\n" for i in range(400))
    st = DocumentState(uri="file:///qupdate.tcl")
    st.update_source_quick(base, 1)
    _ = st.buffer
    cur = base
    for v in range(2, 120):
        cur = cur + f"proc extra{v} {{}} {{ return {v} }}\n"
        st.update_source_quick(cur, v)
        _ = st.buffer
    gc.collect()
    live = set(dict(st._versions))
    # Only the current version (119) should remain resident — older quick-update
    # versions are unpinned and reclaimed.
    assert live == {119}, f"expected only the current version live; got {sorted(live)}"


def _giant_body_interior_offset(source: str) -> int:
    """Offset strictly inside the dominant braced body (the namespace eval body
    in filetypes.tcl) — a brace-safe interior edit point."""
    from compiler.parsing.command_segmenter import segment_top_level_chunks

    chunks = segment_top_level_chunks(source)
    giant = max(chunks, key=lambda c: c.end_offset - c.start_offset)
    body = giant.commands[0].argv[-1]
    return (body.start.offset + 1 + body.end.offset) // 2


@pytest.mark.slow
def test_mvcc_document_path_tens_of_thousands_of_edits_no_leak():
    """Punish the MVCC through the *real document edit path* at scale.

    Drive 20,000 brace-safe interior edits of the 1.3 MB filetypes.tcl through
    update_source_quick — exercising the rope splice + incremental segmentation
    + weak MVCC version registry together (only feasible since the brace-safe
    body-splice fast path made each edit ~5 ms instead of a ~184 ms full re-lex).

    Invariants under the storm:
    * no version leak — periodic GC must keep the resident version count O(1)
      (a strong-ref registry or a buffer cache that never released would retain
      ~20k versions of a >1 MB document, tens of GB, and OOM);
    * byte-identity — the incrementally-maintained chunks stay identical to a
      from-scratch segmentation (sampled periodically, the expensive check);
    * throughput — a loose ceiling flags an O(n)-per-edit regression.
    """
    from compiler.parsing.command_segmenter import segment_top_level_chunks

    source = _big_source(1_000_000)
    assert len(source) >= 1_000_000, "stress test needs a multi-MB source"

    st = DocumentState(uri="file:///stress_doc_filetypes.tcl")
    st.update_source_quick(source, 1)
    _ = st.buffer

    cur = source
    base = _giant_body_interior_offset(source)
    peak_live = len(dict(st._versions))
    n_edits = 20_000
    t0 = time.perf_counter()
    for k in range(2, n_edits + 2):
        pos = base + (k * 131) % 50_000  # rotating interior, brace-safe region
        ins = f"z{k} "
        cur = cur[:pos] + ins + cur[pos:]
        st.update_source_quick(cur, k)
        if k % 2_000 == 0:
            _ = st.buffer  # materialise + exercise the spliced rope
            gc.collect()
            peak_live = max(peak_live, len(dict(st._versions)))
            # Byte-identity: incrementally-maintained chunks == full re-segment.
            assert [c.source_hash for c in st.chunks] == [
                c.source_hash for c in segment_top_level_chunks(cur)
            ], f"incremental chunks diverged from full segmentation at edit {k}"
    elapsed = time.perf_counter() - t0
    gc.collect()
    peak_live = max(peak_live, len(dict(st._versions)))

    assert peak_live <= 4, f"document version registry leaked: peak_live={peak_live}"
    assert st.source == cur and len(cur) > len(source)
    # 20k edits at ~5-6 ms each ≈ 2 min; a generous ceiling catches an
    # O(n)-per-edit (full re-lex) regression that would push it to ~1 h.
    assert elapsed < 360.0, (
        f"20k document edits took {elapsed:.0f}s (per-edit no longer ~O(log n)?)"
    )

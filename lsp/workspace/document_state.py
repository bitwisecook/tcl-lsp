"""Per-document state cache.

Stores the analysis result for each open document so we don't re-analyse
unless the document changes.

Supports incremental updates: when only a subset of top-level chunks
change, cached per-chunk artefacts (IR, analyser snapshot, tokens) are
reused and only dirty chunks are re-processed.

**Threading model**: ``update()`` runs in a background thread.  It does
all expensive work (tokenisation, compilation, analysis, chunk-cache
building) into local variables, then swaps the results into a
``_StateSnapshot`` under a brief lock.  Concurrent readers on the event
loop thread access the snapshot atomically via ``DocumentState.snap``,
guaranteeing they never observe partially-updated state.
"""

from __future__ import annotations

import logging
import threading
import time
from dataclasses import dataclass, field
from typing import Any

from core.analysis.analyser import Analyser, AnalyserSnapshot, AnalysisResult
from core.commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from core.commands.registry.runtime import is_irules_dialect
from core.common.codes import default_disabled_diagnostics
from core.common.document_buffer import DocumentBuffer
from core.compiler.compilation_unit import CompilationUnit, FunctionUnit, compile_source
from core.compiler.interprocedural import ProcLocalSummary
from core.compiler.ir import IRProcedure, IRStatement
from core.compiler.lowering import lower_to_ir
from core.parsing.command_segmenter import (
    TopLevelChunk,
    find_first_dirty_chunk,
    segment_top_level_chunks,
)
from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token

# Lazy import to avoid circular dependencies at module load time.
_style_diag_fn = None
_style_diag_all_fn = None
_precompute_chunk_tokens_fn = None


def _get_style_diag_fn():
    """Lazily import ``compute_style_diagnostics_for_range``."""
    global _style_diag_fn
    if _style_diag_fn is None:
        from lsp.features.diagnostics import compute_style_diagnostics_for_range

        _style_diag_fn = compute_style_diagnostics_for_range
    return _style_diag_fn


def _get_style_diag_all_fn():
    """Lazily import ``compute_all_style_diagnostics``."""
    global _style_diag_all_fn
    if _style_diag_all_fn is None:
        from lsp.features.diagnostics import compute_all_style_diagnostics

        _style_diag_all_fn = compute_all_style_diagnostics
    return _style_diag_all_fn


def _get_precompute_chunk_tokens_fn():
    """Lazily import ``precompute_chunk_tokens``."""
    global _precompute_chunk_tokens_fn
    if _precompute_chunk_tokens_fn is None:
        from lsp.features.semantic_tokens import precompute_chunk_tokens

        _precompute_chunk_tokens_fn = precompute_chunk_tokens
    return _precompute_chunk_tokens_fn


def _chunk_line_range(
    buf: DocumentBuffer,
    chunk: TopLevelChunk,
) -> tuple[int, int, int, int]:
    """O(log n) chunk line range using ``DocumentBuffer``.

    Returns ``(start_line, start_col, end_line, end_col)`` for a chunk.
    """
    return buf.chunk_line_range(chunk.start_offset, chunk.end_offset)


def _extract_chunk_ir(
    compilation_unit: object | None,
    chunks: list[TopLevelChunk],
) -> list[tuple[tuple, dict]] | None:
    """Extract per-chunk IR from an already-compiled ``IRModule``.

    Returns a list parallel to *chunks* of ``(ir_stmts, ir_procs)`` tuples,
    or ``None`` if the compilation unit / IR module is unavailable.  This
    avoids redundant re-lowering of each chunk during cache building.
    """
    if compilation_unit is None:
        return None
    ir_module = getattr(compilation_unit, "ir_module", None)
    if ir_module is None:
        return None
    top_stmts = ir_module.top_level.statements
    all_procs = ir_module.procedures

    # Sort procedures by start offset for O(S + P + C) sliding-window
    # extraction instead of O(C × P).
    sorted_procs = sorted(all_procs.items(), key=lambda p: p[1].range.start.offset)

    result: list[tuple[tuple, dict]] = []
    stmt_idx = 0
    proc_idx = 0
    for chunk in chunks:
        c_start = chunk.start_offset
        c_end = chunk.end_offset
        # Collect top-level statements whose range falls within this chunk.
        chunk_stmts: list = []
        while stmt_idx < len(top_stmts):
            stmt = top_stmts[stmt_idx]
            if stmt.range.start.offset >= c_end:
                break
            if stmt.range.start.offset >= c_start:
                chunk_stmts.append(stmt)
            stmt_idx += 1
        # Collect procedures via sliding pointer (procs sorted by offset).
        chunk_procs: dict = {}
        while proc_idx < len(sorted_procs):
            name, proc = sorted_procs[proc_idx]
            if proc.range.start.offset >= c_end:
                break
            if proc.range.start.offset >= c_start:
                chunk_procs[name] = proc
            proc_idx += 1
        result.append((tuple(chunk_stmts), chunk_procs))
    return result


log = logging.getLogger(__name__)


def _build_proc_cache(
    cu: CompilationUnit,
) -> dict[tuple[str, int], FunctionUnit]:
    """Build a proc cache from a CompilationUnit.

    Keys are ``(qualified_name, hash(proc_source_text))`` so that a
    procedure's FunctionUnit can be reused across edits as long as
    the body text is identical.
    """
    cache: dict[tuple[str, int], FunctionUnit] = {}
    for qname, fu in cu.procedures.items():
        ir_proc = cu.ir_module.procedures.get(qname)
        if ir_proc is not None:
            proc_src = cu.source[ir_proc.range.start.offset : ir_proc.range.end.offset]
            cache[(qname, hash(proc_src))] = fu
    return cache


@dataclass
class ChunkCache:
    """Cached artefacts for a single ``TopLevelChunk``.

    Stored per-chunk so that unchanged chunks can skip re-lowering and
    re-analysis on subsequent edits.
    """

    chunk_hash: int
    ir_statements: tuple[IRStatement, ...]
    procedures: dict[str, IRProcedure]
    # Analyser snapshot taken *after* this chunk was analysed.
    # ``None`` for the very first chunk (no prior state to snapshot).
    analyser_snapshot_after: AnalyserSnapshot | None
    # Cached style diagnostics (W111/W112/W115) — LSP-format diagnostics.
    style_diagnostics: list[Any] | None = None
    # Line length used when computing style_diagnostics, for cache validation.
    style_line_length: int = 120
    # Cached semantic tokens in absolute position format:
    # (line, col, length, type_idx, modifier_bits)
    semantic_tokens_abs: list[tuple[int, int, int, int, int]] | None = None


@dataclass
class _StateSnapshot:
    """Immutable snapshot of all handler-visible ``DocumentState`` fields.

    Built by ``update()`` / ``update_source_quick()`` and swapped
    atomically so concurrent readers never observe torn state.
    """

    source: str = ""
    version: int | None = None
    # Lazily computed on first access via DocumentState.tokens to
    # avoid an O(source_len) lexer pass on every incremental update.
    tokens: list[Token] | None = None
    analysis: AnalysisResult | None = None
    compilation_unit: CompilationUnit | None = None
    chunks: list[TopLevelChunk] = field(default_factory=list)
    has_partial_commands: bool = False
    file_profiles: frozenset[str] = field(default_factory=frozenset)
    chunk_caches: list[ChunkCache | None] = field(default_factory=list)
    buffer: DocumentBuffer | None = None
    deep_diag_proc_key: frozenset[tuple[str, int]] | None = None
    deep_diag_result: list[Any] | None = None


@dataclass
class DocumentState:
    """Cached analysis state for a single document.

    Handler-visible state is stored in a ``_StateSnapshot`` that is
    atomically swapped after each update.  Property accessors
    delegate to the current snapshot, so readers never see
    partially-updated fields.
    """

    uri: str
    language_id: str = ""
    _snap: _StateSnapshot = field(default_factory=_StateSnapshot, repr=False)
    _lock: threading.Lock = field(default_factory=threading.Lock, repr=False)
    # Internal caches for the compilation pipeline — not accessed by
    # request handlers, so they live outside the snapshot.
    _proc_cache: dict[tuple[str, int], FunctionUnit] = field(
        default_factory=dict,
        repr=False,
    )
    _interproc_cache: dict[tuple[str, int], ProcLocalSummary] = field(
        default_factory=dict,
        repr=False,
    )

    # -- Property accessors delegating to the snapshot ----------------
    # Getters provide atomic reads (single attribute load from ``_snap``).
    # **Setters are NOT thread-safe** — they mutate the live snapshot
    # without going through the lock-protected swap path.  They exist
    # only for test/script convenience and MUST NOT be called from
    # request handlers or background threads.  Production code should
    # use ``update()`` / ``update_source_quick()`` which build and swap
    # a complete ``_StateSnapshot`` atomically.

    @property
    def snap(self) -> _StateSnapshot:
        """The current immutable state snapshot."""
        return self._snap

    @property
    def source(self) -> str:
        return self._snap.source

    @source.setter
    def source(self, value: str) -> None:
        self._snap.source = value

    @property
    def version(self) -> int | None:
        return self._snap.version

    @version.setter
    def version(self, value: int | None) -> None:
        self._snap.version = value

    @property
    def tokens(self) -> list[Token]:
        snap = self._snap
        if snap.tokens is None:
            with self._lock:
                # Double-checked locking: re-read after acquiring lock.
                snap = self._snap
                if snap.tokens is None:
                    snap.tokens = TclLexer(snap.source).tokenise_all()
        return snap.tokens

    @tokens.setter
    def tokens(self, value: list[Token]) -> None:
        self._snap.tokens = value

    @property
    def analysis(self) -> AnalysisResult | None:
        return self._snap.analysis

    @analysis.setter
    def analysis(self, value: AnalysisResult | None) -> None:
        self._snap.analysis = value

    @property
    def compilation_unit(self) -> CompilationUnit | None:
        return self._snap.compilation_unit

    @compilation_unit.setter
    def compilation_unit(self, value: CompilationUnit | None) -> None:
        self._snap.compilation_unit = value

    @property
    def chunks(self) -> list[TopLevelChunk]:
        return self._snap.chunks

    @chunks.setter
    def chunks(self, value: list[TopLevelChunk]) -> None:
        self._snap.chunks = value

    @property
    def has_partial_commands(self) -> bool:
        return self._snap.has_partial_commands

    @has_partial_commands.setter
    def has_partial_commands(self, value: bool) -> None:
        self._snap.has_partial_commands = value

    @property
    def file_profiles(self) -> frozenset[str]:
        return self._snap.file_profiles

    @file_profiles.setter
    def file_profiles(self, value: frozenset[str]) -> None:
        self._snap.file_profiles = value

    @property
    def _chunk_caches(self) -> list[ChunkCache | None]:
        return self._snap.chunk_caches

    @_chunk_caches.setter
    def _chunk_caches(self, value: list[ChunkCache | None]) -> None:
        self._snap.chunk_caches = value

    @property
    def _buffer(self) -> DocumentBuffer | None:
        return self._snap.buffer

    @_buffer.setter
    def _buffer(self, value: DocumentBuffer | None) -> None:
        self._snap.buffer = value

    @property
    def _deep_diag_proc_key(self) -> frozenset[tuple[str, int]] | None:
        return self._snap.deep_diag_proc_key

    @_deep_diag_proc_key.setter
    def _deep_diag_proc_key(self, value: frozenset[tuple[str, int]] | None) -> None:
        self._snap.deep_diag_proc_key = value

    @property
    def _deep_diag_result(self) -> list[Any] | None:
        return self._snap.deep_diag_result

    @_deep_diag_result.setter
    def _deep_diag_result(self, value: list[Any] | None) -> None:
        self._snap.deep_diag_result = value

    @property
    def buffer(self) -> DocumentBuffer:
        """Shared position infrastructure for the current source text.

        Lazily created on first access; invalidated whenever ``source``
        changes (by setting ``_buffer = None``).
        """
        snap = self._snap
        if snap.buffer is None or snap.buffer.source is not snap.source:
            snap.buffer = DocumentBuffer.from_source(snap.source, snap.version)
        return snap.buffer

    @property
    def lines(self) -> list[str]:
        """Source split into lines, cached for the lifetime of the current source."""
        return self.buffer.lines

    def get_cached_style_diagnostics(
        self,
        *,
        disabled_diagnostics: set[str] | None = None,
        line_length: int = 120,
    ) -> list[Any] | None:
        """Assemble cached style diagnostics from chunk caches.

        Returns ``None`` if any chunk is missing cached style diagnostics
        or if the line-length setting has changed since the diagnostics
        were cached, meaning the caller should fall back to full
        computation.

        The ``disabled_diagnostics`` filtering is applied downstream by
        ``get_basic_diagnostics``, so we only validate line length here
        (the only setting that affects style diagnostic *computation*).
        """
        if not self._chunk_caches or not self.chunks:
            return None
        if len(self._chunk_caches) != len(self.chunks):
            return None
        result: list[Any] = []
        for cc in self._chunk_caches:
            if cc is None or cc.style_diagnostics is None:
                return None
            if cc.style_line_length != line_length:
                return None
            result.extend(cc.style_diagnostics)
        return result

    def get_semantic_token_cache(
        self,
    ) -> (
        tuple[
            list[list[tuple[int, int, int, int, int]] | None],
            list[tuple[int, int, int, int]],
        ]
        | None
    ):
        """Return ``(chunk_token_cache, chunk_line_ranges)`` for semantic tokens.

        The returned ``chunk_token_cache`` is a mutable list: entries that
        are ``None`` will be populated by ``semantic_tokens_full`` and
        written back to the ``ChunkCache`` objects.

        Each range is ``(start_line, start_col, end_line, end_col)`` so
        that chunks sharing a line get non-overlapping boundaries.
        """
        if not self._chunk_caches or not self.chunks:
            return None
        if len(self._chunk_caches) != len(self.chunks):
            return None

        cache: list[list[tuple[int, int, int, int, int]] | None] = []
        ranges: list[tuple[int, int, int, int]] = []
        buf = self.buffer
        for i, cc in enumerate(self._chunk_caches):
            if cc is None:
                cache.append(None)
            else:
                cache.append(cc.semantic_tokens_abs)
            ranges.append(_chunk_line_range(buf, self.chunks[i]))
        return cache, ranges

    def store_semantic_token_cache(
        self,
        chunk_token_cache: list[list[tuple[int, int, int, int, int]] | None],
    ) -> None:
        """Write back computed semantic tokens to chunk caches."""
        for i, tokens in enumerate(chunk_token_cache):
            if (
                tokens is not None
                and i < len(self._chunk_caches)
                and self._chunk_caches[i] is not None
            ):
                self._chunk_caches[i].semantic_tokens_abs = tokens  # type: ignore[union-attr, invalid-assignment]

    def get_deep_diag_proc_key(self) -> frozenset[tuple[str, int]]:
        """Compute the identity key for deep diagnostic caching.

        This is a frozenset of ``(name, source_hash)`` tuples representing
        all procedures in the current compilation unit, plus a synthetic
        entry for the top-level code so that edits to non-proc code also
        invalidate the cache.
        """
        key_entries: set[tuple[str, int]] = set(self._proc_cache.keys())
        key_entries.add(("__TOPLEVEL__", hash(self.source)))
        return frozenset(key_entries)

    def get_cached_deep_diagnostics(self) -> list[Any] | None:
        """Return cached deep diagnostics if the proc set hasn't changed."""
        if self._deep_diag_result is None or self._deep_diag_proc_key is None:
            return None
        current_key = self.get_deep_diag_proc_key()
        if current_key == self._deep_diag_proc_key:
            return self._deep_diag_result
        return None

    def store_deep_diagnostics(self, diagnostics: list[Any]) -> None:
        """Cache deep diagnostics with the current proc identity key."""
        self._deep_diag_proc_key = self.get_deep_diag_proc_key()
        self._deep_diag_result = diagnostics

    def update_source_quick(
        self,
        source: str,
        version: int | None = None,
    ) -> bool:
        """Fast source-only update: set source/version/chunks, clear analysis.

        Returns ``True`` if the source actually changed and a full
        ``update()`` is needed afterwards to rebuild analysis/caches.
        Returns ``False`` if unchanged (no further work needed).

        This is designed to be called on the event loop before yielding,
        so that semantic token requests can be served immediately with
        the new source text (without analysis enrichment).

        Builds a new ``_StateSnapshot`` and swaps it atomically.
        Chunk caches for unchanged chunks are carried forward so that
        semantic token requests between the quick update and the full
        ``update()`` can reuse cached tokens instead of recomputing
        the entire document.
        """
        old = self._snap
        if source == old.source and (version is None or version == old.version):
            return False  # unchanged
        new_chunks = segment_top_level_chunks(source)
        has_partial = any(cmd.is_partial for chunk in new_chunks for cmd in chunk.commands)
        # Carry forward chunk caches for unchanged chunks so that
        # semantic token requests can serve cached tokens immediately.
        old_caches = old.chunk_caches
        new_caches: list[ChunkCache | None] = []
        if old_caches:
            dirty_idx = find_first_dirty_chunk(old.chunks, new_chunks)
            for i, chunk in enumerate(new_chunks):
                if i < dirty_idx and i < len(old_caches):
                    cc = old_caches[i]
                    if cc is not None and cc.chunk_hash == chunk.source_hash:
                        new_caches.append(cc)
                        continue
                new_caches.append(None)
        self._snap = _StateSnapshot(
            source=source,
            version=version,
            chunks=new_chunks,
            has_partial_commands=has_partial,
            chunk_caches=new_caches,
            file_profiles=old.file_profiles,
        )
        return True

    def update(
        self,
        source: str,
        version: int | None = None,
        *,
        force_reanalyse: bool = False,
        line_length: int = 120,
    ) -> None:
        """Re-analyse if the source has changed.

        Uses chunk-level hashing to detect unchanged prefixes.  When the
        entire chunk list matches (identical hashes), the previous
        analysis result is kept as-is without re-running any pipeline
        stages.

        When a rebuild *is* required, the incremental path reuses cached
        per-chunk IR and analyser snapshots for chunks before the first
        dirty chunk, only re-lowering and re-analysing from the dirty
        point onwards.  Procedure-level caching avoids recomputing SSA
        and dataflow analysis for procs whose source text has not changed.

        **Threading**: All expensive work (tokenisation, compilation,
        analysis, chunk-cache building) runs into local variables.  The
        resulting ``_StateSnapshot`` is swapped into ``self._snap``
        atomically at the end, so concurrent readers on the event-loop
        thread never observe partially-updated state.
        """
        t0 = time.perf_counter()
        # Snapshot the current state for incremental decisions.
        # This read is atomic (single attribute load under the GIL).
        old_snap = self._snap
        # Reuse chunks from update_source_quick() when the source matches,
        # avoiding a redundant O(source_len) lexer pass.
        if source == old_snap.source and old_snap.chunks:
            new_chunks = old_snap.chunks
        else:
            new_chunks = segment_top_level_chunks(source)
        t_seg = time.perf_counter()
        has_partial = any(cmd.is_partial for chunk in new_chunks for cmd in chunk.commands)

        if not force_reanalyse and old_snap.analysis is not None:
            if source == old_snap.source:
                log.debug(
                    "[timing] document update %.0fms (unchanged)",
                    (time.perf_counter() - t0) * 1000,
                )
                return
            dirty_idx = find_first_dirty_chunk(old_snap.chunks, new_chunks)
            if dirty_idx >= len(new_chunks) and dirty_idx >= len(old_snap.chunks):
                # All chunks match — source is semantically identical.
                new_snap = _StateSnapshot(
                    source=source,
                    version=version,
                    tokens=old_snap.tokens,
                    analysis=old_snap.analysis,
                    compilation_unit=old_snap.compilation_unit,
                    chunks=new_chunks,
                    has_partial_commands=has_partial,
                    file_profiles=old_snap.file_profiles,
                    chunk_caches=old_snap.chunk_caches if old_snap.chunks == new_chunks else [],
                    deep_diag_proc_key=old_snap.deep_diag_proc_key,
                    deep_diag_result=old_snap.deep_diag_result,
                )
                self._snap = new_snap
                log.debug(
                    "[timing] document update %.0fms (all chunks match)",
                    (time.perf_counter() - t0) * 1000,
                )
                return

            # Incremental path: try to reuse cached artefacts.
            try:
                self._update_incremental(
                    source,
                    version,
                    old_snap,
                    new_chunks,
                    has_partial,
                    dirty_idx,
                    line_length=line_length,
                )
                log.info(
                    "[timing] document update %.0fms (incremental, dirty=%d/%d, segment=%.0fms)",
                    (time.perf_counter() - t0) * 1000,
                    dirty_idx,
                    len(new_chunks),
                    (t_seg - t0) * 1000,
                )
                return
            except Exception:
                log.debug(
                    "document_state: incremental update failed, falling back to full rebuild",
                    exc_info=True,
                )
                # Fall through to full rebuild.

        self._update_full(source, version, new_chunks, has_partial, line_length=line_length)
        log.info(
            "[timing] document update %.0fms (full rebuild, %d chunks, segment=%.0fms)",
            (time.perf_counter() - t0) * 1000,
            len(new_chunks),
            (t_seg - t0) * 1000,
        )

    def _update_incremental(
        self,
        source: str,
        version: int | None,
        old_snap: _StateSnapshot,
        new_chunks: list[TopLevelChunk],
        has_partial: bool,
        dirty_idx: int,
        *,
        line_length: int = 120,
    ) -> None:
        """Incremental update: reuse cached artefacts for clean chunks.

        Builds all new state into local variables and swaps a new
        ``_StateSnapshot`` atomically at the end.
        """
        t0 = time.perf_counter()
        file_profiles = (
            EVENT_REGISTRY.compute_file_profiles(source) if is_irules_dialect() else frozenset()
        )
        # Tokens are computed lazily on first access via DocumentState.tokens,
        # avoiding an O(source_len) lexer pass when no handler needs them.
        t_tok = time.perf_counter()

        # Build chunk IR cache: reuse cached entries for clean chunks,
        # leave dirty chunks as None so lower_to_ir will process them.
        chunk_ir: list[tuple[tuple[IRStatement, ...], dict[str, IRProcedure]] | None] = []
        new_chunk_caches: list[ChunkCache | None] = []
        old_chunk_caches = old_snap.chunk_caches

        for i, chunk in enumerate(new_chunks):
            if i < dirty_idx and i < len(old_chunk_caches):
                cached = old_chunk_caches[i]
                if cached is not None and cached.chunk_hash == chunk.source_hash:
                    chunk_ir.append((cached.ir_statements, cached.procedures))
                    new_chunk_caches.append(cached)
                    continue
            # Dirty or no cache — will be lowered fresh.
            chunk_ir.append(None)
            new_chunk_caches.append(None)

        # Lower IR incrementally.
        prev_proc_cache = dict(self._proc_cache)
        prev_interproc_cache = dict(self._interproc_cache)
        ir_module = None
        compilation_unit: CompilationUnit | None = None
        try:
            ir_module = lower_to_ir(source, chunk_ir=chunk_ir, chunks=new_chunks)
            t_lower = time.perf_counter()
            compilation_unit = compile_source(
                source,
                ir_module=ir_module,
                proc_cache=self._proc_cache,
                interproc_cache=self._interproc_cache,
                prune_interproc_cache=not has_partial,
            )
        except Exception:
            t_lower = time.perf_counter()
            log.debug("document_state: incremental compilation failed", exc_info=True)
            self._proc_cache = prev_proc_cache
            self._interproc_cache = prev_interproc_cache
        t_compile = time.perf_counter()

        self._do_update_proc_cache(compilation_unit, has_partial)

        # Incremental analysis: restore from the last clean chunk's
        # analyser snapshot and analyse dirty chunks with per-chunk
        # snapshots in a single pass (avoiding double analysis).
        restore_snapshot: AnalyserSnapshot | None = None
        if dirty_idx > 0:
            for i in range(dirty_idx - 1, -1, -1):
                cc = new_chunk_caches[i] if i < len(new_chunk_caches) else None
                if cc is not None and cc.analyser_snapshot_after is not None:
                    restore_snapshot = cc.analyser_snapshot_after
                    break

        dirty_chunk_commands = [list(chunk.commands) for chunk in new_chunks[dirty_idx:]]

        if restore_snapshot is not None:
            analyser = Analyser(disabled_diagnostics=default_disabled_diagnostics())
            analyser.restore(restore_snapshot)
            analysis, dirty_snapshots = analyser.analyse_chunked(
                source,
                dirty_chunk_commands,
                cu=compilation_unit,
                skip_stubs=True,
            )
        else:
            # No snapshot to restore from — full chunked analysis.
            analyser = Analyser(disabled_diagnostics=default_disabled_diagnostics())
            analysis, dirty_snapshots = analyser.analyse_chunked(
                source,
                dirty_chunk_commands,
                cu=compilation_unit,
            )
        t_analyse = time.perf_counter()

        # Build chunk caches for dirty chunks using the snapshots from
        # analyse_chunked — no re-analysis needed.
        if compilation_unit is None and ir_module is None:
            log.debug("Skipping chunk-cache update: incremental compilation failed")
        else:
            self._build_dirty_chunk_caches(
                source,
                new_chunks,
                new_chunk_caches,
                dirty_idx,
                dirty_snapshots,
                ir_module,
                compilation_unit,
                line_length=line_length,
                analysis=analysis,
            )
        t_caches = time.perf_counter()

        # Atomic swap: build and install the new snapshot.
        # Tokens are left as None (lazy) — computed on first access.
        self._snap = _StateSnapshot(
            source=source,
            version=version,
            analysis=analysis,
            compilation_unit=compilation_unit,
            chunks=new_chunks,
            has_partial_commands=has_partial,
            file_profiles=file_profiles,
            chunk_caches=new_chunk_caches,
        )

        log.info(
            "[timing] _update_incremental lower=%.0fms compile=%.0fms"
            " analyse=%.0fms caches=%.0fms total=%.0fms (dirty=%d/%d)",
            (t_lower - t_tok) * 1000,
            (t_compile - t_lower) * 1000,
            (t_analyse - t_compile) * 1000,
            (t_caches - t_analyse) * 1000,
            (t_caches - t0) * 1000,
            dirty_idx,
            len(new_chunks),
        )

    def _build_dirty_chunk_caches(
        self,
        source: str,
        chunks: list[TopLevelChunk],
        chunk_caches: list[ChunkCache | None],
        dirty_idx: int,
        dirty_snapshots: list[AnalyserSnapshot],
        ir_module: object | None,
        compilation_unit: CompilationUnit | None,
        *,
        line_length: int = 120,
        analysis: AnalysisResult | None = None,
    ) -> None:
        """Build ``ChunkCache`` entries for dirty chunks using pre-built snapshots.

        Unlike the old ``_rebuild_chunk_caches_for_dirty``, this does **not**
        re-run the analyser — it uses snapshots captured by ``analyse_chunked``
        during the single analysis pass.

        When *analysis* is provided, semantic tokens are pre-computed for
        dirty chunks so the next ``semanticTokens/full`` gets a cache hit.
        """
        try:
            from bisect import bisect_left, bisect_right

            buf = DocumentBuffer.from_source(source)
            all_style_diags = _get_style_diag_all_fn()(source, line_length=line_length)
            diag_lines = [d.range.start.line for d in all_style_diags]

            # Extract per-chunk IR from the IR module when available.
            dirty_chunks = chunks[dirty_idx:]
            chunk_ir_map = _extract_chunk_ir(compilation_unit, dirty_chunks)

            for di, chunk in enumerate(dirty_chunks):
                i = dirty_idx + di
                snap = dirty_snapshots[di]

                # Use pre-extracted IR if available; fall back to lowering.
                if chunk_ir_map is not None:
                    ir_stmts, ir_procs = chunk_ir_map[di]
                else:
                    from core.compiler.lowering import lower_commands_to_ir

                    ir_stmts, ir_procs = lower_commands_to_ir(source, list(chunk.commands))

                # Partition pre-computed style diagnostics for this chunk.
                start_line, _sc, end_line, _ec = _chunk_line_range(buf, chunk)
                lo = bisect_left(diag_lines, start_line)
                hi = bisect_right(diag_lines, end_line)
                style_diags = all_style_diags[lo:hi]

                # Extend chunk_caches to cover this index.
                while len(chunk_caches) <= i:
                    chunk_caches.append(None)

                chunk_caches[i] = ChunkCache(
                    chunk_hash=chunk.source_hash,
                    ir_statements=ir_stmts,
                    procedures=ir_procs,
                    analyser_snapshot_after=snap,
                    style_diagnostics=style_diags,
                    style_line_length=line_length,
                )

            # Pre-compute semantic tokens for dirty chunks so the next
            # semanticTokens/full request gets a cache hit.
            try:
                chunk_line_ranges = [_chunk_line_range(buf, c) for c in chunks]
                uri_lower = self.uri.lower()
                is_irules = uri_lower.endswith(".irul") or uri_lower.endswith(".irule")
                chunk_toks = _get_precompute_chunk_tokens_fn()(
                    source,
                    chunk_line_ranges,
                    analysis=analysis,
                    is_irules=is_irules,
                )
                for ci in range(dirty_idx, len(chunks)):
                    cc = chunk_caches[ci] if ci < len(chunk_caches) else None
                    if cc is not None and ci < len(chunk_toks):
                        cc.semantic_tokens_abs = chunk_toks[ci]
            except Exception:
                log.debug(
                    "document_state: semantic token precompute failed (dirty)",
                    exc_info=True,
                )
        except Exception:
            log.debug(
                "document_state: failed to build chunk caches for dirty region",
                exc_info=True,
            )

    def _update_full(
        self,
        source: str,
        version: int | None,
        new_chunks: list[TopLevelChunk],
        has_partial: bool,
        *,
        line_length: int = 120,
    ) -> None:
        """Full rebuild — no incremental reuse.

        Builds all state into local variables and swaps a new
        ``_StateSnapshot`` atomically at the end.
        """
        t0 = time.perf_counter()
        file_profiles = (
            EVENT_REGISTRY.compute_file_profiles(source) if is_irules_dialect() else frozenset()
        )
        # Tokens are computed lazily on first access via DocumentState.tokens.
        t_tok = time.perf_counter()

        prev_proc_cache = dict(self._proc_cache)
        prev_interproc_cache = dict(self._interproc_cache)
        compilation_unit: CompilationUnit | None = None
        try:
            compilation_unit = compile_source(
                source,
                proc_cache=self._proc_cache,
                interproc_cache=self._interproc_cache,
                prune_interproc_cache=not has_partial,
            )
        except Exception:
            log.debug("document_state: compilation failed, preserving caches", exc_info=True)
            self._proc_cache = prev_proc_cache
            self._interproc_cache = prev_interproc_cache
        t_compile = time.perf_counter()

        self._do_update_proc_cache(compilation_unit, has_partial)

        # Analyse and build chunk caches in a single pass: process commands
        # chunk-by-chunk, capturing snapshots at each boundary.  This avoids
        # the old pattern of running analyse() then re-analysing per-chunk.
        chunk_commands = [list(chunk.commands) for chunk in new_chunks]
        analyser = Analyser(disabled_diagnostics=default_disabled_diagnostics())
        analysis, chunk_snapshots = analyser.analyse_chunked(
            source,
            chunk_commands,
            cu=compilation_unit,
        )
        t_analyse = time.perf_counter()

        # Build chunk caches using pre-built snapshots.
        # Pass analysis so semantic tokens are pre-computed per chunk,
        # eliminating the redundant full-document lex on the first
        # semanticTokens/full request.
        chunk_caches = self._build_full_chunk_caches(
            source,
            new_chunks,
            chunk_snapshots,
            compilation_unit,
            line_length=line_length,
            analysis=analysis,
        )
        t_caches = time.perf_counter()

        # Atomic swap: install the new snapshot.
        # Tokens left as None (lazy) — computed on first access.
        self._snap = _StateSnapshot(
            source=source,
            version=version,
            analysis=analysis,
            compilation_unit=compilation_unit,
            chunks=new_chunks,
            has_partial_commands=has_partial,
            file_profiles=file_profiles,
            chunk_caches=chunk_caches,
        )

        n_procs = len(compilation_unit.procedures) if compilation_unit else 0
        log.info(
            "[timing] _update_full compile=%.0fms analyse=%.0fms"
            " chunk_caches=%.0fms total=%.0fms (procs=%d)",
            (t_compile - t_tok) * 1000,
            (t_analyse - t_compile) * 1000,
            (t_caches - t_analyse) * 1000,
            (t_caches - t0) * 1000,
            n_procs,
        )

    def _build_full_chunk_caches(
        self,
        source: str,
        chunks: list[TopLevelChunk],
        chunk_snapshots: list[AnalyserSnapshot] | None,
        compilation_unit: CompilationUnit | None,
        *,
        line_length: int = 120,
        analysis: AnalysisResult | None = None,
    ) -> list[ChunkCache | None]:
        """Build ``ChunkCache`` entries after a full rebuild.

        When *chunk_snapshots* is provided (from ``analyse_chunked``),
        the snapshots are used directly — no re-analysis is performed.

        When *analysis* is provided, semantic tokens are pre-computed
        for each chunk so that the first ``semanticTokens/full`` request
        after analysis gets a full cache hit without re-lexing.

        Returns the list of chunk caches (does not write to ``self``).
        """
        t0 = time.perf_counter()
        try:
            from bisect import bisect_left, bisect_right

            buf = DocumentBuffer.from_source(source)
            caches: list[ChunkCache | None] = []

            # Compute style diagnostics once for the whole file, then
            # partition by chunk using bisect (O(lines) + O(chunks×log(diags))
            # instead of the old O(chunks×lines) approach).
            all_style_diags = _get_style_diag_all_fn()(source, line_length=line_length)
            diag_lines = [d.range.start.line for d in all_style_diags]

            # Extract per-chunk IR from the already-compiled IRModule when
            # available, avoiding redundant re-lowering of each chunk.
            chunk_ir_map = _extract_chunk_ir(compilation_unit, chunks)

            # Fall back to per-chunk analysis if no pre-built snapshots.
            snapshot_analyser: Analyser | None = None
            if chunk_snapshots is None:
                snapshot_analyser = Analyser()
                snapshot_analyser._source = source

            for ci, chunk in enumerate(chunks):
                # Use pre-extracted IR if available; fall back to lowering.
                if chunk_ir_map is not None:
                    ir_stmts, ir_procs = chunk_ir_map[ci]
                else:
                    from core.compiler.lowering import lower_commands_to_ir

                    ir_stmts, ir_procs = lower_commands_to_ir(source, list(chunk.commands))

                # Use pre-built snapshot or analyse this chunk for one.
                if chunk_snapshots is not None:
                    snap = chunk_snapshots[ci]
                else:
                    assert snapshot_analyser is not None
                    snapshot_analyser._analyse_commands_inner(
                        list(chunk.commands),
                        snapshot_analyser._current_scope,
                        source,
                    )
                    snap = snapshot_analyser.snapshot()

                # Partition pre-computed style diagnostics for this chunk.
                start_line, _sc, end_line, _ec = _chunk_line_range(buf, chunk)
                lo = bisect_left(diag_lines, start_line)
                hi = bisect_right(diag_lines, end_line)
                style_diags = all_style_diags[lo:hi]

                caches.append(
                    ChunkCache(
                        chunk_hash=chunk.source_hash,
                        ir_statements=ir_stmts,
                        procedures=ir_procs,
                        analyser_snapshot_after=snap,
                        style_diagnostics=style_diags,
                        style_line_length=line_length,
                    )
                )
            # Pre-compute semantic tokens per chunk so the first
            # semanticTokens/full request gets a full cache hit.
            try:
                chunk_line_ranges = [_chunk_line_range(buf, c) for c in chunks]
                uri_lower = self.uri.lower()
                is_irules = uri_lower.endswith(".irul") or uri_lower.endswith(".irule")
                chunk_toks = _get_precompute_chunk_tokens_fn()(
                    source,
                    chunk_line_ranges,
                    analysis=analysis,
                    is_irules=is_irules,
                )
                for ci, cc in enumerate(caches):
                    if cc is not None and ci < len(chunk_toks):
                        cc.semantic_tokens_abs = chunk_toks[ci]
            except Exception:
                log.debug(
                    "document_state: semantic token precompute failed",
                    exc_info=True,
                )
            result = caches
        except Exception:
            log.debug("document_state: failed to build chunk caches", exc_info=True)
            result = []
        log.info(
            "[timing] _build_full_chunk_caches %.0fms (%d chunks)",
            (time.perf_counter() - t0) * 1000,
            len(chunks),
        )
        return result

    def _do_update_proc_cache(
        self,
        compilation_unit: CompilationUnit | None,
        has_partial: bool,
    ) -> None:
        """Update the procedure cache from the given compilation unit."""
        if compilation_unit is not None:
            next_proc_cache = _build_proc_cache(compilation_unit)
            if has_partial:
                merged = dict(self._proc_cache)
                merged.update(next_proc_cache)
                self._proc_cache = merged
            else:
                self._proc_cache = next_proc_cache
        else:
            if not has_partial:
                self._proc_cache = {}
                self._interproc_cache = {}


class WorkspaceState:
    """Manages DocumentState objects for all open documents."""

    def __init__(self) -> None:
        self._documents: dict[str, DocumentState] = {}

    def get(self, uri: str) -> DocumentState | None:
        return self._documents.get(uri)

    def open(
        self,
        uri: str,
        source: str,
        version: int | None = None,
        *,
        language_id: str = "",
        force_reanalyse: bool = False,
        analyse: bool = True,
        line_length: int = 120,
    ) -> DocumentState:
        """Register a newly opened document.

        Parameters
        ----------
        analyse:
            When *False*, only store the source text and metadata without
            running the analysis pipeline.  The caller is responsible for
            triggering analysis later (e.g. via ``_publish_diagnostics``
            running in a background thread).  This keeps ``didOpen`` fast
            so the event loop remains responsive for other requests.
        line_length:
            Maximum line length for style diagnostics.
        """
        state = DocumentState(uri=uri, language_id=language_id)
        if analyse:
            state.update(source, version, force_reanalyse=force_reanalyse, line_length=line_length)
        else:
            # Lightweight open: store source and version without analysis.
            # Swap a fresh snapshot so no mutable setter touches a live one.
            state._snap = _StateSnapshot(source=source, version=version)
        self._documents[uri] = state
        return state

    def update(
        self,
        uri: str,
        source: str,
        version: int | None = None,
        *,
        force_reanalyse: bool = False,
        line_length: int = 120,
    ) -> DocumentState:
        state = self._documents.get(uri)
        if state is None:
            return self.open(
                uri,
                source,
                version,
                force_reanalyse=force_reanalyse,
                line_length=line_length,
            )
        state.update(source, version, force_reanalyse=force_reanalyse, line_length=line_length)
        return state

    def close(self, uri: str) -> None:
        self._documents.pop(uri, None)

    def get_language_id(self, uri: str) -> str:
        """Return the ``language_id`` from the editor for *uri*, or ``""``."""
        state = self._documents.get(uri)
        return state.language_id if state is not None else ""

    def items(self) -> list[tuple[str, DocumentState]]:
        """Return all open documents."""
        return list(self._documents.items())

"""Per-document state cache.

Stores the analysis result for each open document so we don't re-analyse
unless the document changes.

Supports incremental updates: when only a subset of top-level chunks
change, cached per-chunk artefacts (IR, analyser snapshot, tokens) are
reused and only dirty chunks are re-processed.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from typing import Any

from core.analysis.analyser import Analyser, AnalyserSnapshot, AnalysisResult, analyse
from core.commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from core.commands.registry.runtime import is_irules_dialect
from core.compiler.compilation_unit import CompilationUnit, FunctionUnit, compile_source
from core.compiler.interprocedural import ProcLocalSummary
from core.compiler.ir import IRProcedure, IRStatement
from core.compiler.lowering import lower_to_ir
from core.parsing.command_segmenter import (
    SegmentedCommand,
    TopLevelChunk,
    find_first_dirty_chunk,
    segment_top_level_chunks,
)
from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token

# Lazy import to avoid circular dependencies at module load time.
_style_diag_fn = None


def _get_style_diag_fn():
    """Lazily import ``compute_style_diagnostics_for_range``."""
    global _style_diag_fn
    if _style_diag_fn is None:
        from lsp.features.diagnostics import compute_style_diagnostics_for_range

        _style_diag_fn = compute_style_diagnostics_for_range
    return _style_diag_fn


def _chunk_line_range(source: str, chunk: TopLevelChunk) -> tuple[int, int, int, int]:
    """Return ``(start_line, start_col, end_line, end_col)`` for a chunk.

    Uses ``(line, col)`` positions (not just line numbers) so that
    semicolon-separated chunks sharing the same line get non-overlapping
    boundaries and can be cached independently.

    ``end_col`` points one past the last character of the chunk (exclusive).
    """
    prefix = source[: chunk.start_offset]
    start_line = prefix.count("\n")
    last_nl = prefix.rfind("\n")
    start_col = chunk.start_offset - (last_nl + 1)

    end_prefix = source[: chunk.end_offset]
    end_line = end_prefix.count("\n")
    last_nl_end = end_prefix.rfind("\n")
    end_col = chunk.end_offset - (last_nl_end + 1)
    return start_line, start_col, end_line, end_col


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
class DocumentState:
    """Cached analysis state for a single document."""

    uri: str
    language_id: str = ""
    version: int | None = None
    source: str = ""
    tokens: list[Token] = field(default_factory=list)
    analysis: AnalysisResult | None = None
    compilation_unit: CompilationUnit | None = None
    chunks: list[TopLevelChunk] = field(default_factory=list)
    has_partial_commands: bool = False
    file_profiles: frozenset[str] = field(default_factory=frozenset)
    _lines: list[str] | None = field(default=None, repr=False)
    _proc_cache: dict[tuple[str, int], FunctionUnit] = field(
        default_factory=dict,
        repr=False,
    )
    _interproc_cache: dict[tuple[str, int], ProcLocalSummary] = field(
        default_factory=dict,
        repr=False,
    )
    _chunk_caches: list[ChunkCache | None] = field(
        default_factory=list,
        repr=False,
    )
    _deep_diag_proc_key: frozenset[tuple[str, int]] | None = field(
        default=None,
        repr=False,
    )
    _deep_diag_result: list[Any] | None = field(
        default=None,
        repr=False,
    )

    @property
    def lines(self) -> list[str]:
        """Source split into lines, cached for the lifetime of the current source."""
        if self._lines is None:
            self._lines = self.source.split("\n")
        return self._lines

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
        for i, cc in enumerate(self._chunk_caches):
            if cc is None:
                cache.append(None)
            else:
                cache.append(cc.semantic_tokens_abs)
            ranges.append(_chunk_line_range(self.source, self.chunks[i]))
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
                self._chunk_caches[i].semantic_tokens_abs = tokens  # type: ignore[union-attr]

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

    def update(
        self,
        source: str,
        version: int | None = None,
        *,
        force_reanalyse: bool = False,
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
        """
        new_chunks = segment_top_level_chunks(source)
        has_partial = any(cmd.is_partial for chunk in new_chunks for cmd in chunk.commands)

        if not force_reanalyse and self.analysis is not None:
            if source == self.source:
                return
            dirty_idx = find_first_dirty_chunk(self.chunks, new_chunks)
            if dirty_idx >= len(new_chunks) and dirty_idx >= len(self.chunks):
                # All chunks match — source is semantically identical.
                self.source = source
                self._lines = None
                self.version = version
                # Invalidate per-chunk caches when offsets may have shifted
                # (e.g. whitespace changes between commands).  Token ranges
                # and diagnostic positions are offset-dependent, so stale
                # caches would produce incorrect results.
                if self.chunks != new_chunks:
                    self._chunk_caches = []
                self.chunks = new_chunks
                self.has_partial_commands = has_partial
                return

            # Incremental path: try to reuse cached artefacts.
            try:
                self._update_incremental(
                    source,
                    version,
                    new_chunks,
                    has_partial,
                    dirty_idx,
                )
                return
            except Exception:
                log.debug(
                    "document_state: incremental update failed, falling back to full rebuild",
                    exc_info=True,
                )
                # Fall through to full rebuild.

        self._update_full(source, version, new_chunks, has_partial)

    def _update_incremental(
        self,
        source: str,
        version: int | None,
        new_chunks: list[TopLevelChunk],
        has_partial: bool,
        dirty_idx: int,
    ) -> None:
        """Incremental update: reuse cached artefacts for clean chunks."""
        self.chunks = new_chunks
        self.has_partial_commands = has_partial
        self.source = source
        self._lines = None
        self.version = version
        self.file_profiles = (
            EVENT_REGISTRY.compute_file_profiles(source) if is_irules_dialect() else frozenset()
        )
        self.tokens = TclLexer(source).tokenise_all()

        # Build chunk IR cache: reuse cached entries for clean chunks,
        # leave dirty chunks as None so lower_to_ir will process them.
        chunk_ir: list[tuple[tuple[IRStatement, ...], dict[str, IRProcedure]] | None] = []
        new_chunk_caches: list[ChunkCache | None] = []

        for i, chunk in enumerate(new_chunks):
            if i < dirty_idx and i < len(self._chunk_caches):
                cached = self._chunk_caches[i]
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
        try:
            ir_module = lower_to_ir(source, chunk_ir=chunk_ir, chunks=new_chunks)
            self.compilation_unit = compile_source(
                source,
                ir_module=ir_module,
                proc_cache=self._proc_cache,
                interproc_cache=self._interproc_cache,
                prune_interproc_cache=not self.has_partial_commands,
            )
        except Exception:
            log.debug("document_state: incremental compilation failed", exc_info=True)
            self.compilation_unit = None
            self._proc_cache = prev_proc_cache
            self._interproc_cache = prev_interproc_cache

        self._update_proc_cache()

        # Incremental analysis: restore from the last clean chunk's
        # analyser snapshot and only analyse commands from dirty chunks.
        restore_snapshot: AnalyserSnapshot | None = None
        if dirty_idx > 0:
            for i in range(dirty_idx - 1, -1, -1):
                cc = new_chunk_caches[i] if i < len(new_chunk_caches) else None
                if cc is not None and cc.analyser_snapshot_after is not None:
                    restore_snapshot = cc.analyser_snapshot_after
                    break

        # Gather commands from dirty chunks onwards.
        dirty_commands: list[SegmentedCommand] = []
        for chunk in new_chunks[dirty_idx:]:
            dirty_commands.extend(chunk.commands)

        if restore_snapshot is not None:
            analyser = Analyser()
            analyser.restore(restore_snapshot)
            self.analysis = analyser.analyse_commands(
                source,
                dirty_commands,
                cu=self.compilation_unit,
            )
        else:
            # No snapshot to restore from — full analysis.
            self.analysis = analyse(source, cu=self.compilation_unit)

        # Update chunk caches for newly processed chunks.
        # Take analyser snapshots at each dirty chunk boundary.
        # If incremental compilation failed (no ir_module despite expecting
        # one), skip chunk cache updates to avoid storing empty IR that
        # could later be mistaken for valid cached entries.
        if self.compilation_unit is None and ir_module is None:
            log.debug("Skipping chunk-cache update: incremental compilation failed")
            self._chunk_caches = new_chunk_caches
        else:
            self._rebuild_chunk_caches_for_dirty(
                source,
                new_chunks,
                new_chunk_caches,
                dirty_idx,
                ir_module if self.compilation_unit else None,
            )
            self._chunk_caches = new_chunk_caches

    def _rebuild_chunk_caches_for_dirty(
        self,
        source: str,
        chunks: list[TopLevelChunk],
        chunk_caches: list[ChunkCache | None],
        dirty_idx: int,
        ir_module: object | None,
    ) -> None:
        """Build ``ChunkCache`` entries for newly-analysed dirty chunks.

        Takes analyser snapshots at each chunk boundary so future
        incremental passes can restore from the last clean point.
        """
        # To build snapshots, we need to re-run the analyser per-chunk.
        # This is a bookkeeping pass — the actual analysis results have
        # already been computed.  We only need the snapshots.
        try:
            snapshot_analyser = Analyser()
            # If there's a clean snapshot before the dirty region, start from there.
            restored = False
            if dirty_idx > 0:
                for i in range(dirty_idx - 1, -1, -1):
                    cc = chunk_caches[i] if i < len(chunk_caches) else None
                    if cc is not None and cc.analyser_snapshot_after is not None:
                        snapshot_analyser.restore(cc.analyser_snapshot_after)
                        restored = True
                        break

            if not restored and dirty_idx > 0:
                # Process clean chunks to build the snapshot base.
                all_clean_cmds: list[SegmentedCommand] = []
                for chunk in chunks[:dirty_idx]:
                    all_clean_cmds.extend(chunk.commands)
                snapshot_analyser._source = source
                snapshot_analyser._analyse_commands_inner(
                    all_clean_cmds,
                    snapshot_analyser._current_scope,
                    source,
                )

            # Compute style diagnostics for dirty chunks.
            style_fn = _get_style_diag_fn()

            # Now process each dirty chunk and take snapshots.
            for i in range(dirty_idx, len(chunks)):
                chunk = chunks[i]
                cmds = list(chunk.commands)
                snapshot_analyser._source = source
                snapshot_analyser._analyse_commands_inner(
                    cmds,
                    snapshot_analyser._current_scope,
                    source,
                )
                snap = snapshot_analyser.snapshot()

                # Build IR cache entry for this chunk from the IR module.
                ir_stmts: tuple[IRStatement, ...] = ()
                ir_procs: dict[str, IRProcedure] = {}
                if ir_module is not None and hasattr(ir_module, "top_level"):
                    from core.compiler.ir import IRModule as IRModuleType

                    if isinstance(ir_module, IRModuleType):
                        # We can't easily extract per-chunk IR from the assembled
                        # module, so re-lower this chunk's commands.
                        from core.compiler.lowering import lower_commands_to_ir

                        ir_stmts, ir_procs = lower_commands_to_ir(source, cmds)

                # Style diagnostics for this chunk's line range.
                start_line, _sc, end_line, _ec = _chunk_line_range(source, chunk)
                style_diags = style_fn(source, start_line, end_line)

                # Extend chunk_caches to cover this index.
                while len(chunk_caches) <= i:
                    chunk_caches.append(None)

                chunk_caches[i] = ChunkCache(
                    chunk_hash=chunk.source_hash,
                    ir_statements=ir_stmts,
                    procedures=ir_procs,
                    analyser_snapshot_after=snap,
                    style_diagnostics=style_diags,
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
    ) -> None:
        """Full rebuild — no incremental reuse."""
        self.chunks = new_chunks
        self.has_partial_commands = has_partial

        self.source = source
        self._lines = None
        self.version = version
        self.file_profiles = (
            EVENT_REGISTRY.compute_file_profiles(source) if is_irules_dialect() else frozenset()
        )
        self.tokens = TclLexer(source).tokenise_all()

        prev_proc_cache = dict(self._proc_cache)
        prev_interproc_cache = dict(self._interproc_cache)
        try:
            self.compilation_unit = compile_source(
                source,
                proc_cache=self._proc_cache,
                interproc_cache=self._interproc_cache,
                prune_interproc_cache=not self.has_partial_commands,
            )
        except Exception:
            log.debug("document_state: compilation failed, preserving caches", exc_info=True)
            self.compilation_unit = None
            self._proc_cache = prev_proc_cache
            self._interproc_cache = prev_interproc_cache

        self._update_proc_cache()
        self.analysis = analyse(source, cu=self.compilation_unit)

        # Build chunk caches for future incremental use.
        self._build_full_chunk_caches(source, new_chunks)

    def _build_full_chunk_caches(
        self,
        source: str,
        chunks: list[TopLevelChunk],
    ) -> None:
        """Build ``ChunkCache`` entries after a full rebuild."""
        try:
            from core.compiler.lowering import lower_commands_to_ir

            caches: list[ChunkCache | None] = []
            snapshot_analyser = Analyser()
            snapshot_analyser._source = source

            # Compute style diagnostics per chunk.
            style_fn = _get_style_diag_fn()

            for chunk in chunks:
                cmds = list(chunk.commands)
                # Lower per-chunk IR.
                ir_stmts, ir_procs = lower_commands_to_ir(source, cmds)
                # Analyse commands for snapshot.
                snapshot_analyser._analyse_commands_inner(
                    cmds,
                    snapshot_analyser._current_scope,
                    source,
                )
                snap = snapshot_analyser.snapshot()

                # Style diagnostics for this chunk's line range.
                start_line, _sc, end_line, _ec = _chunk_line_range(source, chunk)
                style_diags = style_fn(source, start_line, end_line)

                caches.append(
                    ChunkCache(
                        chunk_hash=chunk.source_hash,
                        ir_statements=ir_stmts,
                        procedures=ir_procs,
                        analyser_snapshot_after=snap,
                        style_diagnostics=style_diags,
                    )
                )
            self._chunk_caches = caches
        except Exception:
            log.debug("document_state: failed to build chunk caches", exc_info=True)
            self._chunk_caches = []

    def _update_proc_cache(self) -> None:
        """Update the procedure cache from the current compilation unit."""
        if self.compilation_unit is not None:
            next_proc_cache = _build_proc_cache(self.compilation_unit)
            if self.has_partial_commands:
                merged = dict(self._proc_cache)
                merged.update(next_proc_cache)
                self._proc_cache = merged
            else:
                self._proc_cache = next_proc_cache
        else:
            if not self.has_partial_commands:
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
    ) -> DocumentState:
        state = DocumentState(uri=uri, language_id=language_id)
        state.update(source, version, force_reanalyse=force_reanalyse)
        self._documents[uri] = state
        return state

    def update(
        self,
        uri: str,
        source: str,
        version: int | None = None,
        *,
        force_reanalyse: bool = False,
    ) -> DocumentState:
        state = self._documents.get(uri)
        if state is None:
            return self.open(uri, source, version, force_reanalyse=force_reanalyse)
        state.update(source, version, force_reanalyse=force_reanalyse)
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

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
import weakref
from dataclasses import dataclass, field, replace
from typing import TYPE_CHECKING, Any

from analyser import Analyser, AnalyserSnapshot, AnalysisResult
from compiler.compilation_unit import CompilationUnit, FunctionUnit, compile_source
from compiler.interprocedural import ProcLocalSummary
from compiler.ir import IRProcedure, IRStatement
from compiler.lowering import lower_to_ir
from compiler.parsing.command_segmenter import (
    TopLevelChunk,
    find_first_dirty_chunk,
    segment_top_level_chunks,
)
from compiler.parsing.green_tree import tokenise
from compiler.parsing.incremental import (
    EditRange,
    incremental_top_level_chunks,
    infer_edit_range,
)
from compiler.registry.dialect import detect_dialect_from_source, dialect_scope
from compiler.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from compiler.registry.runtime import is_irules_dialect
from shared.codes import default_disabled_diagnostics
from shared.document_buffer import DocumentBuffer
from shared.rope import RopeEdit
from shared.tokens import Token

if TYPE_CHECKING:
    from analyser.semantic_model import WorkspaceDiagnosticContext
    from server.features.incremental_diagnostics import ProcDiagEntry

# Lazy import to avoid circular dependencies at module load time.
_style_diag_fn = None
_style_diag_all_fn = None
_precompute_chunk_tokens_fn = None


def _get_style_diag_fn():
    """Lazily import ``compute_style_diagnostics_for_range``."""
    global _style_diag_fn
    if _style_diag_fn is None:
        from server.features.diagnostics import compute_style_diagnostics_for_range

        _style_diag_fn = compute_style_diagnostics_for_range
    return _style_diag_fn


def _get_style_diag_all_fn():
    """Lazily import ``compute_all_style_diagnostics``."""
    global _style_diag_all_fn
    if _style_diag_all_fn is None:
        from server.features.diagnostics import compute_all_style_diagnostics

        _style_diag_all_fn = compute_all_style_diagnostics
    return _style_diag_all_fn


def _owned_style_diags(
    all_style_diags: list[Any],
    diag_lines: list[int],
    start_line: int,
    end_line: int,
    consumed: int,
) -> tuple[list[Any], int]:
    """Style diagnostics owned by one chunk, advancing a monotonic cursor.

    Whole-line style diagnostics (W111/W112/W115) are computed once for the
    file and partitioned to chunks by inclusive line range.  When several
    top-level chunks share a physical line (``cmd1 ; cmd2``), a plain per-chunk
    bisect slice hands the *same* line-level diagnostic to every chunk on that
    line, so ``get_cached_style_diagnostics`` then publishes it N times.  The
    ``consumed`` cursor (exclusive end index already handed out, with chunks
    visited in document order) makes each diagnostic land in exactly the first
    chunk whose ``end_line`` reaches it.  Returns ``(style_diags, new_consumed)``.
    """
    from bisect import bisect_left, bisect_right

    lo = max(consumed, bisect_left(diag_lines, start_line))
    hi = bisect_right(diag_lines, end_line)
    style = all_style_diags[lo:hi] if hi > lo else []
    return style, max(consumed, hi)


def _get_precompute_chunk_tokens_fn():
    """Lazily import ``precompute_chunk_tokens``."""
    global _precompute_chunk_tokens_fn
    if _precompute_chunk_tokens_fn is None:
        from server.features import precompute_chunk_tokens

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

    # Sort procedures by start offset for a single-pass merge with
    # chunks — O(procs*log(procs) + chunks + procs) instead of the
    # old O(chunks * procs) nested scan.
    sorted_procs = sorted(all_procs.items(), key=lambda kv: kv[1].range.start.offset)

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

_IRULES_EXTENSIONS = (".irul", ".irule")
_IAPPS_EXTENSIONS = (".iapp", ".iappimpl", ".impl", ".apl")
_EXPECT_EXTENSIONS = (".exp",)


def _effective_disabled_diagnostics(uri: str | None) -> frozenset[str]:
    """Effective ``disabled_diagnostics`` for the analyser at *uri*.

    The analyser gates expensive opt-in checks (W123 unresolved-command
    suggestions, W242 loop termination, W307/W308 indirect-call checks,
    etc.) on its ``_disabled_diagnostics`` set.  Constructing the
    analyser with the static ``default_disabled_diagnostics()`` made it
    impossible for users to enable opt-in codes via
    ``tclLsp.diagnostics.<code>: true`` (the user's choice was filtered
    only *post*-analysis by ``get_basic_diagnostics`` — but the analyser
    had already short-circuited the emit path).  Resolving the
    per-folder ``FeatureConfig`` here keeps the gate honest.

    Done as a lazy import to break the ``lsp.state`` ↔
    ``lsp.workspace.document_state`` import cycle.
    """
    from server.state import config_for_uri

    cfg = config_for_uri(uri)
    return frozenset(cfg.disabled_diagnostics)


def infer_document_dialect(uri: str, source: str, language_id: str = "") -> str | None:
    """Infer the best dialect hint for a single document."""
    lang = language_id.lower()
    if lang in {"irules", "f5-irules", "tcl-irules"}:
        return "f5-irules"
    if lang in {"iapps", "f5-iapps", "tcl-iapps", "apl", "tcl-apl", "apl-lang"}:
        return "f5-iapps"
    if lang == "expect":
        return "expect"

    basename = uri.rsplit("/", 1)[-1].lower() if "/" in uri else uri.lower()
    if basename.endswith(_IRULES_EXTENSIONS):
        return "f5-irules"
    if basename.endswith(_IAPPS_EXTENSIONS) or basename == "presentation":
        return "f5-iapps"
    if basename.endswith(_EXPECT_EXTENSIONS):
        return "expect"

    # BIG-IP configuration files (bigip.conf, bigip_base.conf, …) are
    # not Tcl source — they are key-value config stanzas.  Resolve
    # their dialect to ``"f5-bigip"`` so the general Tcl analysis
    # pipeline knows to skip them.
    from server.workspace.scanner import _BIGIP_CONF_NAMES

    if basename in _BIGIP_CONF_NAMES:
        return "f5-bigip"

    return detect_dialect_from_source(source)


# ---------------------------------------------------------------------------
# Subprocess worker function for ProcessPoolExecutor
# ---------------------------------------------------------------------------


def _analyse_document_fresh(
    source: str,
    version: int | None,
    line_length: int,
    dialect: str,
    uri: str,
    disabled_diagnostics: set[str] | None = None,
    disabled_optimisations: set[str] | None = None,
    optimiser_enabled: bool = True,
    extra_commands: tuple[str, ...] = (),
    non_ascii_mode: str | None = None,
    stub_commands: tuple = (),
    line_ending: str = "\n",
    workspace_context: "WorkspaceDiagnosticContext | None" = None,
) -> dict:
    """Run the full analysis pipeline in a subprocess.

    This is a module-level function (picklable) that replicates
    ``_update_full`` but returns a result dict instead of mutating
    ``DocumentState``.  Called via ``ProcessPoolExecutor`` to escape
    the GIL and achieve true parallelism.

    The subprocess does not share ContextVars with the parent task, so
    every per-request setting that the analyser / checks read from a
    ContextVar (dialect, extra_commands, W108 non-ASCII mode) must be
    forwarded explicitly and re-applied here.  See issue #407.
    """
    # BIG-IP configuration files (bigip.conf, bigip_base.conf, …) are
    # not Tcl source — they are key-value config stanzas that may
    # embed Tcl fragments (tmsh, iApp APL, iRules).  The general Tcl
    # analyser must never be run on their top-level text; doing so
    # misinterprets BIG-IP encrypted-string markers ($M$…$) as Tcl
    # variable references.  Diagnostics and semantic features for
    # these files are handled by the bigip-specific parser and
    # validator in
    # ``server.diagnostics_pipeline._publish_bigip_diagnostics``.
    if dialect == "f5-bigip":
        return {
            "analysis": None,
            "compilation_unit": None,
            "chunks": [],
            "has_partial": False,
            "chunk_caches": [],
            "file_profiles": frozenset(),
            "buffer": DocumentBuffer.from_source(source, version),
            "basic_diags": [],
            "suppressed": {},
            "conf_wrapped": False,
            "embedded_rules": [],
        }

    # Ensure diagnostic codes are registered in the subprocess.
    import server._codes_init  # noqa: F401
    from compiler.registry.runtime import configure_signatures

    configure_signatures(dialect=dialect, extra_commands=list(extra_commands))
    if non_ascii_mode is not None:
        from analyser.checks._style import set_non_ascii_mode

        set_non_ascii_mode(non_ascii_mode)

    # Re-establish workspace .tcl.stubs in this worker.  Set (not scoped):
    # the pool reuses workers and forwards stubs on every call, mirroring
    # configure_signatures above.
    from compiler.registry.stub_comments import set_ambient_stubs

    set_ambient_stubs(stub_commands)

    # The analyser gates expensive opt-in checks (W123 unresolved-command
    # suggestions, W242 loop termination, etc.) on its
    # ``_disabled_diagnostics`` set.  Use the *effective* set from the
    # parent's ``feature_config`` (already filtered for user enablement)
    # rather than ``default_disabled_diagnostics()`` so opt-in codes the
    # user has enabled via ``tclLsp.diagnostics.<code>: true`` actually
    # reach the emit path.  See issue #407 follow-up.
    analyser_disabled = (
        frozenset(disabled_diagnostics)
        if disabled_diagnostics is not None
        else default_disabled_diagnostics()
    )

    t0 = time.perf_counter()

    chunks = segment_top_level_chunks(source)
    has_partial = any(cmd.is_partial for chunk in chunks for cmd in chunk.commands)

    file_profiles: frozenset[str] = frozenset()
    try:
        from compiler.registry.runtime import is_irules_dialect

        if is_irules_dialect():
            file_profiles = EVENT_REGISTRY.compute_file_profiles(source)
    except Exception:
        pass

    # Conf-wrapped iRules: analyse each rule body independently.
    from dialects.f5.bigip.rule_extract import is_conf_wrapped_irules

    if is_irules_dialect() and is_conf_wrapped_irules(source):
        from analyser.conf_wrapped import analyse_conf_wrapped

        analysis, embedded_rules = analyse_conf_wrapped(
            source,
            disabled_diagnostics=analyser_disabled,
            file_path=uri,
        )
        all_profiles: set[str] = set()
        for rule in embedded_rules:
            all_profiles.update(EVENT_REGISTRY.compute_file_profiles(rule.body))
        file_profiles = frozenset(all_profiles) | file_profiles

        buf = DocumentBuffer.from_source(source, version)
        basic_diags = []
        suppressed: dict[int, frozenset[str]] = {}
        try:
            from server.features.diagnostics import get_basic_diagnostics

            basic_diags, _analysis_out, suppressed = get_basic_diagnostics(
                source,
                analysis=analysis,
                cu=None,
                optimiser_enabled=False,
                disabled_diagnostics=disabled_diagnostics or set(),
                disabled_optimisations=disabled_optimisations or set(),
                line_length=line_length,
                line_ending=line_ending,
                cached_style_diagnostics=None,
                workspace_context=workspace_context,
                uri=uri,
            )
        except Exception:
            log.debug("subprocess: conf-wrapped basic diagnostics failed", exc_info=True)

        elapsed_ms = (time.perf_counter() - t0) * 1000
        log.info(
            "[timing] _analyse_document_fresh (conf-wrapped) %.0fms (rules=%d)",
            elapsed_ms,
            len(embedded_rules),
        )
        return {
            "analysis": analysis,
            "compilation_unit": None,
            "chunks": chunks,
            "has_partial": has_partial,
            "chunk_caches": [],
            "file_profiles": file_profiles,
            "buffer": buf,
            "basic_diags": basic_diags,
            "suppressed": suppressed,
            "conf_wrapped": True,
            "embedded_rules": embedded_rules,
        }

    compilation_unit: CompilationUnit | None = None
    try:
        compilation_unit = compile_source(source)
    except Exception:
        log.debug("subprocess: compilation failed", exc_info=True)

    chunk_commands = [list(chunk.commands) for chunk in chunks]
    analyser = Analyser(disabled_diagnostics=analyser_disabled)
    analysis, chunk_snapshots = analyser.analyse_chunked(
        source,
        chunk_commands,
        cu=compilation_unit,
        file_path=uri,
    )

    # Determine dialect flags for semantic token precompute.
    uri_lower = uri.lower() if uri else ""
    is_irules_file = uri_lower.endswith(".irul") or uri_lower.endswith(".irule")

    chunk_caches = _build_chunk_caches_standalone(
        source,
        chunks,
        chunk_snapshots,
        compilation_unit,
        line_length=line_length,
        analysis=analysis,
        is_irules=is_irules_file or is_irules_dialect(),
        is_bigip_conf=uri_lower.endswith(".conf"),
        is_apl=uri_lower.endswith(".apl"),
    )

    buf = DocumentBuffer.from_source(source, version)

    # Compute basic diagnostics in the subprocess so the main process
    # doesn't need to run _phase1 in asyncio.to_thread (which would
    # hold the GIL and block the event loop).
    basic_diags = []
    suppressed: dict[int, frozenset[str]] = {}
    try:
        from server.features.diagnostics import get_basic_diagnostics

        partial = any(cmd.is_partial for chunk in chunks for cmd in chunk.commands)
        basic_diags, _analysis_out, suppressed = get_basic_diagnostics(
            source,
            analysis=analysis,
            cu=compilation_unit,
            optimiser_enabled=optimiser_enabled and not partial,
            disabled_diagnostics=disabled_diagnostics or set(),
            disabled_optimisations=disabled_optimisations or set(),
            line_length=line_length,
            line_ending=line_ending,
            cached_style_diagnostics=None,
            # Forwarded from the parent so the cold build produces the same
            # workspace-aware diagnostics as the in-thread phase1: line-ending
            # checks (line_ending) and W120/W123 workspace filtering
            # (workspace_context).  WorkspaceDiagnosticContext is a small,
            # picklable bundle of frozensets/dicts of strings.
            workspace_context=workspace_context,
            uri=uri,
        )
    except Exception:
        log.debug("subprocess: basic diagnostics failed", exc_info=True)

    elapsed_ms = (time.perf_counter() - t0) * 1000
    n_procs = len(compilation_unit.procedures) if compilation_unit else 0
    log.info(
        "[timing] _analyse_document_fresh %.0fms (procs=%d, chunks=%d, lines=%d)",
        elapsed_ms,
        n_procs,
        len(chunks),
        source.count("\n") + 1,
    )

    return {
        "analysis": analysis,
        "compilation_unit": compilation_unit,
        "chunks": chunks,
        "has_partial": has_partial,
        "chunk_caches": chunk_caches,
        "file_profiles": file_profiles,
        "buffer": buf,
        "basic_diags": basic_diags,
        "suppressed": suppressed,
    }


def _build_chunk_caches_standalone(
    source: str,
    chunks: list[TopLevelChunk],
    chunk_snapshots: list[AnalyserSnapshot] | None,
    compilation_unit: CompilationUnit | None,
    *,
    line_length: int = 120,
    analysis: AnalysisResult | None = None,
    is_irules: bool = False,
    is_bigip_conf: bool = False,
    is_apl: bool = False,
) -> list[ChunkCache | None]:
    """Build chunk caches including semantic token precompute.

    Standalone version of ``DocumentState._build_full_chunk_caches``
    suitable for subprocess execution (no ``self`` dependency).
    """
    buf = DocumentBuffer.from_source(source)
    caches: list[ChunkCache | None] = []

    all_style_diags = _get_style_diag_all_fn()(source, line_length=line_length)
    diag_lines = [d.range.start.line for d in all_style_diags]
    style_consumed = 0

    chunk_ir_map = _extract_chunk_ir(compilation_unit, chunks)

    snapshot_analyser: Analyser | None = None
    if chunk_snapshots is None:
        snapshot_analyser = Analyser()
        snapshot_analyser._source = source

    for ci, chunk in enumerate(chunks):
        if chunk_ir_map is not None:
            ir_stmts, ir_procs = chunk_ir_map[ci]
        else:
            from compiler.lowering import lower_commands_to_ir

            ir_stmts, ir_procs = lower_commands_to_ir(source, list(chunk.commands))

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

        start_line, _sc, end_line, _ec = _chunk_line_range(buf, chunk)
        style_diags, style_consumed = _owned_style_diags(
            all_style_diags, diag_lines, start_line, end_line, style_consumed
        )

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
        time.sleep(0)  # Yield GIL between chunks

    # Precompute semantic tokens per chunk so the first
    # semanticTokens/full request gets a full cache hit.
    try:
        chunk_line_ranges = [_chunk_line_range(buf, c) for c in chunks]
        chunk_toks = _get_precompute_chunk_tokens_fn()(
            source,
            chunk_line_ranges,
            analysis=analysis,
            is_irules=is_irules,
            is_bigip_conf=is_bigip_conf,
            is_apl=is_apl,
        )
        for ci, cc in enumerate(caches):
            if cc is not None and ci < len(chunk_toks):
                cc.semantic_tokens_abs = chunk_toks[ci]
    except Exception:
        log.debug("_build_chunk_caches_standalone: token precompute failed", exc_info=True)

    return caches


def _build_proc_caches(
    cu: CompilationUnit,
) -> tuple[dict[tuple[str, int], FunctionUnit], dict[tuple[str, int], FunctionUnit]]:
    """Build both the primary proc cache and the reposition cache in one pass.

    Returns ``(proc_cache, reposition_cache)``.  Both caches key the same set of
    ``FunctionUnit``s by validity fingerprints that must match
    :func:`compiler.compilation_unit._proc_cache_key` /
    ``_proc_reposition_key`` exactly; the only difference is that the reposition
    key omits the proc's start line/char/offset (so a moved-but-otherwise-
    unchanged proc still hits it).

    The expensive shared inputs — stub fingerprint, CFG-construction context
    (``prepare_cfg_context`` + ``cfg_context_fingerprint``), known-class
    fingerprint, and the per-proc call-site-constants fingerprints — are
    computed once here and fed into both keys, rather than walking the IR twice
    (one pass per cache).
    """
    from compiler.cfg import cfg_context_fingerprint, prepare_cfg_context
    from compiler.compilation_unit import (
        _proc_cache_key,
        _proc_reposition_key,
        call_site_constants_fingerprint,
        compute_stub_fingerprint,
        known_classes_fingerprint,
    )
    from compiler.core_analyses import _collect_call_site_constants

    proc_cache: dict[tuple[str, int], FunctionUnit] = {}
    reposition_cache: dict[tuple[str, int], FunctionUnit] = {}

    stub_fingerprint = compute_stub_fingerprint(cu.source)
    # The CFG-construction context (which callees write back into the caller
    # frame via upvar, and proc param maps) is part of a cached unit's validity:
    # a caller's CFG/analysis can change when a *callee*'s upvar behaviour
    # changes even though the caller's own text is unchanged.  Must match
    # _proc_cache_key in compilation_unit.py exactly, or a callee edit would
    # leave callers reusing stale units.
    upvar_procs, proc_params = prepare_cfg_context(cu.ir_module)
    context_fingerprint = cfg_context_fingerprint(upvar_procs, proc_params)
    known_classes_fp = known_classes_fingerprint(cu.known_classes)
    # Per-proc call-site-constants fingerprint: a caller editing a literal arg
    # (``foo 1`` → ``foo 0``) changes the proc's SCCP / unreachable-branch facts
    # even though the proc's own text and position are unchanged, so both keys
    # fold it in or a restamped proc would serve stale branch facts.
    call_site_constants_fps = call_site_constants_fingerprint(
        _collect_call_site_constants(cu.ir_module)
    )
    for qname, fu in cu.procedures.items():
        ir_proc = cu.ir_module.procedures.get(qname)
        if ir_proc is None:
            continue
        cs_fp = call_site_constants_fps.get(qname, 0)
        start = ir_proc.range.start
        end_offset = ir_proc.range.end.offset
        key = _proc_cache_key(
            cu.source,
            qname,
            start.offset,
            end_offset,
            stub_fingerprint=stub_fingerprint,
            context_fingerprint=context_fingerprint,
            start_line=start.line,
            start_char=start.character,
            known_classes_fp=known_classes_fp,
            call_site_constants_fp=cs_fp,
        )
        if key is not None:
            proc_cache[key] = fu
        repos_key = _proc_reposition_key(
            cu.source,
            qname,
            start.offset,
            end_offset,
            stub_fingerprint=stub_fingerprint,
            context_fingerprint=context_fingerprint,
            known_classes_fp=known_classes_fp,
            call_site_constants_fp=cs_fp,
        )
        if repos_key is not None:
            reposition_cache[repos_key] = fu
    return proc_cache, reposition_cache


def _build_proc_cache(cu: CompilationUnit) -> dict[tuple[str, int], FunctionUnit]:
    """Primary proc cache only — thin wrapper over :func:`_build_proc_caches`.

    The production update path uses ``_build_proc_caches`` directly (one IR
    pass for both caches); this wrapper exists for callers/tests that only need
    the primary cache.
    """
    return _build_proc_caches(cu)[0]


def _build_reposition_cache(cu: CompilationUnit) -> dict[tuple[str, int], FunctionUnit]:
    """Reposition cache only — thin wrapper over :func:`_build_proc_caches`."""
    return _build_proc_caches(cu)[1]


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
    _tokens: list[Token] | None = field(default=None, repr=False)
    analysis: AnalysisResult | None = None
    compilation_unit: CompilationUnit | None = None
    chunks: list[TopLevelChunk] = field(default_factory=list)
    has_partial_commands: bool = False
    file_profiles: frozenset[str] = field(default_factory=frozenset)
    chunk_caches: list[ChunkCache | None] = field(default_factory=list)
    buffer: DocumentBuffer | None = None
    deep_diag_proc_key: frozenset[tuple[str, int]] | None = None
    deep_diag_result: list[Any] | None = None
    # Per-proc body-local (shimmer) diagnostic cache, keyed (qname, body_hash);
    # the leaf tier of the incremental query-DAG, updated by the deep worker.
    proc_diag_cache: dict[tuple[str, int], ProcDiagEntry] | None = None
    # Conf-wrapped iRules mode: file contains ``ltm rule`` / ``gtm rule``
    # stanzas rather than bare iRule bodies.
    conf_wrapped: bool = False
    embedded_rules: list[Any] = field(default_factory=list)


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
    dialect_hint: str | None = None
    _snap: _StateSnapshot = field(default_factory=_StateSnapshot, repr=False)
    _lock: threading.RLock = field(default_factory=threading.RLock, repr=False)
    # Internal caches for the compilation pipeline — not accessed by
    # request handlers, so they live outside the snapshot.
    _proc_cache: dict[tuple[str, int], FunctionUnit] = field(
        default_factory=dict,
        repr=False,
    )
    # Position-independent companion to ``_proc_cache``: lets a proc that merely
    # moved (a primary-cache miss because its absolute positions shifted) reuse
    # its cached dataflow analysis while rebuilding only the cheap CFG + SSA.
    _reposition_cache: dict[tuple[str, int], FunctionUnit] = field(
        default_factory=dict,
        repr=False,
    )
    _interproc_cache: dict[tuple[str, int], ProcLocalSummary] = field(
        default_factory=dict,
        repr=False,
    )
    # The last fully-analysed snapshot, preserved when ``update_source_quick``
    # swaps in a source-only (analysis-cleared) snapshot.  ``update()`` uses it
    # as the incremental base so a quick update — which exposes new source for
    # immediate token requests — does not force the next analysis down the cold
    # ``_analyse_document_fresh`` path.
    _prev_analysed: _StateSnapshot | None = field(default=None, repr=False)
    # The dialect the currently-cached analysis (live or ``_prev_analysed``) was
    # built under.  ``update_source_quick`` refreshes ``dialect_hint`` eagerly,
    # so ``update()`` must compare the new source's dialect against *this* —
    # not against ``dialect_hint`` — to notice a source-level dialect change.
    _analysed_dialect: str | None = field(default=None, repr=False)
    # MVCC version registry: version -> that version's rope-backed buffer, held
    # *weakly*.  The single live buffer all consumers should read is whichever
    # version is current; an in-flight reader (request handler / analysis task)
    # that captured an older version keeps it alive only while it holds it, and
    # Python's GC reclaims any version no longer referenced — the immutable rope
    # means a still-pinned older version shares structure with the current one,
    # so coexisting in-flight versions are cheap.  Never holds a strong ref, so
    # it cannot itself leak old versions.
    _versions: weakref.WeakValueDictionary[int, DocumentBuffer] = field(
        default_factory=lambda: weakref.WeakValueDictionary(), repr=False
    )

    def _register_version(self, buf: DocumentBuffer | None) -> None:
        """Record *buf* in the weak MVCC registry (no-op for None / no version)."""
        if buf is not None and isinstance(buf.version, int):
            self._versions[buf.version] = buf

    def buffer_for_version(self, version: int) -> DocumentBuffer | None:
        """The rope-backed buffer for document *version*, if still live.

        Held weakly: returns ``None`` once every in-flight reader of that
        version has dropped it (GC reclaimed it).  Lets background / in-flight
        work look a version up without pinning it — the caller pins it for the
        duration of its work simply by holding the returned buffer."""
        return self._versions.get(version)

    def _swap_snapshot(self, snapshot: _StateSnapshot) -> None:
        """Install a fully-built state snapshot under the document lock."""
        with self._lock:
            self._snap = snapshot
            self._register_version(snapshot.buffer)

    def _replace_snapshot(self, **changes: Any) -> None:
        """Replace the current snapshot with selected fields changed."""
        with self._lock:
            self._snap = replace(self._snap, **changes)
            self._register_version(self._snap.buffer)

    def refresh_dialect_hint(self, source: str) -> None:
        """Refresh the per-document dialect hint from metadata and source."""
        self.dialect_hint = infer_document_dialect(self.uri, source, self.language_id)

    def _signature_profile(self) -> Any:
        """Return a context manager for this document's dialect hint."""
        if self.dialect_hint is None:
            return dialect_scope()
        return dialect_scope(self.dialect_hint)

    # -- Property accessors delegating to the snapshot ----------------
    # Getters provide atomic reads (single attribute load from ``_snap``).
    # Setters exist for tests and scripts; they replace the current snapshot
    # under the document lock so readers do not observe in-place mutations.

    @property
    def snap(self) -> _StateSnapshot:
        """The current immutable state snapshot."""
        return self._snap

    @property
    def can_analyse_incrementally(self) -> bool:
        """True when an analysed base exists to build the next analysis on.

        Either the live snapshot is analysed, or a prior analysed snapshot was
        preserved by ``update_source_quick``.  The diagnostics pipeline consults
        this so a quick-update (which clears ``analysis``) does not send a warm
        edit through the cold ``_analyse_document_fresh`` path.
        """
        return self._snap.analysis is not None or (
            self._prev_analysed is not None and self._prev_analysed.analysis is not None
        )

    @property
    def source(self) -> str:
        return self._snap.source

    @source.setter
    def source(self, value: str) -> None:
        self._replace_snapshot(source=value, _tokens=None, buffer=None)

    @property
    def version(self) -> int | None:
        return self._snap.version

    @version.setter
    def version(self, value: int | None) -> None:
        self._replace_snapshot(version=value)

    @property
    def tokens(self) -> list[Token]:
        snap = self._snap
        if snap._tokens is not None:
            return snap._tokens
        with self._lock:
            snap = self._snap
            if snap._tokens is None:
                with self._signature_profile():
                    tokens = list(tokenise(snap.source, 0, 0, 0)[0])
                self._snap = replace(snap, _tokens=tokens)
                return tokens
            return snap._tokens

    @tokens.setter
    def tokens(self, value: list[Token]) -> None:
        self._replace_snapshot(_tokens=value)

    @property
    def analysis(self) -> AnalysisResult | None:
        return self._snap.analysis

    @analysis.setter
    def analysis(self, value: AnalysisResult | None) -> None:
        self._replace_snapshot(analysis=value)

    @property
    def compilation_unit(self) -> CompilationUnit | None:
        return self._snap.compilation_unit

    @compilation_unit.setter
    def compilation_unit(self, value: CompilationUnit | None) -> None:
        self._replace_snapshot(compilation_unit=value)

    @property
    def chunks(self) -> list[TopLevelChunk]:
        return self._snap.chunks

    @chunks.setter
    def chunks(self, value: list[TopLevelChunk]) -> None:
        self._replace_snapshot(chunks=value)

    @property
    def has_partial_commands(self) -> bool:
        return self._snap.has_partial_commands

    @has_partial_commands.setter
    def has_partial_commands(self, value: bool) -> None:
        self._replace_snapshot(has_partial_commands=value)

    @property
    def file_profiles(self) -> frozenset[str]:
        return self._snap.file_profiles

    @file_profiles.setter
    def file_profiles(self, value: frozenset[str]) -> None:
        self._replace_snapshot(file_profiles=value)

    @property
    def _chunk_caches(self) -> list[ChunkCache | None]:
        return self._snap.chunk_caches

    @_chunk_caches.setter
    def _chunk_caches(self, value: list[ChunkCache | None]) -> None:
        self._replace_snapshot(chunk_caches=value)

    @property
    def _buffer(self) -> DocumentBuffer | None:
        return self._snap.buffer

    @_buffer.setter
    def _buffer(self, value: DocumentBuffer | None) -> None:
        self._replace_snapshot(buffer=value)

    @property
    def conf_wrapped(self) -> bool:
        """Whether this document is a conf-wrapped iRules file."""
        return self._snap.conf_wrapped

    @property
    def embedded_rules(self) -> list:
        """The list of ``EmbeddedRule`` objects for conf-wrapped files."""
        return self._snap.embedded_rules

    @property
    def _deep_diag_proc_key(self) -> frozenset[tuple[str, int]] | None:
        return self._snap.deep_diag_proc_key

    @_deep_diag_proc_key.setter
    def _deep_diag_proc_key(self, value: frozenset[tuple[str, int]] | None) -> None:
        self._replace_snapshot(deep_diag_proc_key=value)

    @property
    def _deep_diag_result(self) -> list[Any] | None:
        return self._snap.deep_diag_result

    @_deep_diag_result.setter
    def _deep_diag_result(self, value: list[Any] | None) -> None:
        self._replace_snapshot(deep_diag_result=value)

    def get_proc_diag_cache(self) -> dict[tuple[str, int], ProcDiagEntry] | None:
        """Per-proc body-local diagnostic cache from the previous deep pass."""
        return self._snap.proc_diag_cache

    def store_proc_diag_cache(self, cache: dict[tuple[str, int], ProcDiagEntry]) -> None:
        """Persist the per-proc body-local diagnostic cache for the next edit."""
        self._replace_snapshot(proc_diag_cache=cache)

    @property
    def buffer(self) -> DocumentBuffer:
        """Shared position infrastructure for the current source text.

        Lazily created on first access; invalidated whenever ``source``
        changes (by setting ``_buffer = None``).
        """
        snap = self._snap
        if snap.buffer is None or snap.buffer.source != snap.source:
            with self._lock:
                snap = self._snap
                if snap.buffer is None or snap.buffer.source != snap.source:
                    buffer = DocumentBuffer.from_source(snap.source, snap.version)
                    self._snap = replace(snap, buffer=buffer)
                    self._register_version(buffer)
                    return buffer
        return snap.buffer

    def _carry_or_build_buffer(self, source: str, version: int | None) -> DocumentBuffer:
        """Reuse the current snapshot's rope-backed buffer when it already
        matches *source* — ``update_source_quick`` built it by splicing the edit
        into the prior rope — instead of rebuilding the O(n) position index for
        the full-analysis snapshot.  Refreshes only the version when the text is
        identical but the version advanced; builds fresh when nothing reusable."""
        buf = self._snap.buffer
        if buf is not None and buf.source == source:
            if buf.version == version:
                return buf
            return DocumentBuffer(source=source, version=version, rope=buf.rope)
        return DocumentBuffer.from_source(source, version)

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
        snap: _StateSnapshot | None = None,
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

        Pass *snap* to read from a specific snapshot (so a handler that captured
        ``state.snap`` once stays consistent with the source/analysis it read
        from the same snapshot); defaults to the live snapshot.
        """
        snap = snap if snap is not None else self._snap
        chunk_caches = snap.chunk_caches
        chunks = snap.chunks
        if not chunk_caches or not chunks:
            return None
        if len(chunk_caches) != len(chunks):
            return None

        buf = snap.buffer if snap.buffer is not None else DocumentBuffer.from_source(snap.source)
        cache: list[list[tuple[int, int, int, int, int]] | None] = []
        ranges: list[tuple[int, int, int, int]] = []
        for i, cc in enumerate(chunk_caches):
            if cc is None:
                cache.append(None)
            else:
                cache.append(cc.semantic_tokens_abs)
            ranges.append(_chunk_line_range(buf, chunks[i]))
        return cache, ranges

    def store_semantic_token_cache(
        self,
        chunk_token_cache: list[list[tuple[int, int, int, int, int]] | None],
        computed_for: _StateSnapshot | None = None,
    ) -> None:
        """Write back computed semantic tokens to chunk caches.

        *computed_for* is the snapshot the tokens were computed against.  A
        concurrent edit may have swapped the live snapshot since; a token list
        is written into chunk *i* only when that chunk's text is unchanged
        (its ``source_hash`` still matches the one the tokens were computed for),
        so stale tokens are never grafted onto a different chunk by index.
        """
        with self._lock:
            snap = self._snap
            src_chunks = computed_for.chunks if computed_for is not None else snap.chunks
            chunk_caches = list(snap.chunk_caches)
            cur_chunks = snap.chunks
            changed = False
            for i, tokens in enumerate(chunk_token_cache):
                if tokens is None or i >= len(chunk_caches):
                    continue
                cache = chunk_caches[i]
                if cache is None:
                    continue
                # Only write if chunk i is the same text the tokens were built
                # for and the cache entry still matches the live chunk.
                if i >= len(cur_chunks) or i >= len(src_chunks):
                    continue
                if (
                    src_chunks[i].source_hash != cur_chunks[i].source_hash
                    or cache.chunk_hash != cur_chunks[i].source_hash
                ):
                    continue
                chunk_caches[i] = replace(cache, semantic_tokens_abs=tokens)
                changed = True
            if changed:
                self._snap = replace(snap, chunk_caches=chunk_caches)

    def apply_subprocess_result(self, result: dict, version: int | None) -> None:
        """Apply the result from ``_analyse_document_fresh`` (subprocess).

        Builds and swaps a new ``_StateSnapshot`` from the subprocess
        output, and seeds ``_proc_cache`` so subsequent incremental
        edits benefit from procedure-level caching.
        """
        cu = result.get("compilation_unit")
        has_partial = result.get("has_partial", False)
        self._do_update_proc_cache(cu, has_partial)
        self._swap_snapshot(
            _StateSnapshot(
                source=result.get("source", self._snap.source),
                version=version,
                analysis=result.get("analysis"),
                compilation_unit=cu,
                chunks=result.get("chunks", []),
                has_partial_commands=has_partial,
                file_profiles=result.get("file_profiles", frozenset()),
                chunk_caches=result.get("chunk_caches", []),
                buffer=result.get("buffer"),
                conf_wrapped=result.get("conf_wrapped", False),
                embedded_rules=result.get("embedded_rules", []),
            )
        )
        # The fresh analysis ran under the document's current dialect; record it
        # so the next warm edit doesn't needlessly force a full rebuild (and a
        # genuine dialect switch is still detected) in update().
        self._analysed_dialect = self.dialect_hint

    def precompute_syntax_tokens(
        self,
        *,
        is_irules: bool = False,
        is_bigip_conf: bool = False,
        is_apl: bool = False,
    ) -> bool:
        """Eagerly precompute syntax-only semantic tokens.

        Builds minimal ``ChunkCache`` entries with just token data so
        that ``get_semantic_token_cache()`` returns a full cache hit
        before the heavy analysis pipeline runs.  Returns ``True`` if
        tokens were precomputed, ``False`` if skipped (cache exists or
        chunks unavailable).

        This runs on the event loop (~200 ms for large files) and is
        overwritten when ``_update_full`` builds analysis-enriched
        caches later.
        """
        snap = self._snap
        if snap.chunk_caches or not snap.chunks or snap.buffer is None:
            return False  # already have caches, or no chunks to work from

        from server.features import precompute_chunk_tokens

        t0 = time.perf_counter()
        buf = snap.buffer
        chunk_ranges = [_chunk_line_range(buf, chunk) for chunk in snap.chunks]
        with self._signature_profile():
            token_lists = precompute_chunk_tokens(
                snap.source,
                chunk_ranges,
                analysis=None,
                is_bigip_conf=is_bigip_conf,
                is_irules=is_irules,
                is_apl=is_apl,
            )
        # Build minimal chunk caches with just token data.
        caches: list[ChunkCache | None] = []
        for i, chunk in enumerate(snap.chunks):
            cc = ChunkCache(
                chunk_hash=chunk.source_hash,
                ir_statements=(),
                procedures={},
                analyser_snapshot_after=None,
                semantic_tokens_abs=token_lists[i] if i < len(token_lists) else None,
            )
            caches.append(cc)
        # Atomic swap: readers see either old (no caches) or new (with tokens).
        self._swap_snapshot(replace(snap, chunk_caches=caches))
        log.info(
            "[timing] precompute_syntax_tokens %.0fms (tokens=%d, chunks=%d)",
            (time.perf_counter() - t0) * 1000,
            sum(len(t) for t in token_lists),
            len(snap.chunks),
        )
        return True

    def get_deep_diag_proc_key(self) -> frozenset[tuple[str, int]]:
        """Compute the identity key for deep diagnostic caching.

        This is a frozenset of ``(name, source_hash)`` tuples representing
        all procedures in the current compilation unit, plus a synthetic
        entry for the top-level code so that edits to non-proc code also
        invalidate the cache.

        **Intra-process only:** the entries use salted ``hash()`` (via the
        ``_proc_cache`` keys and ``hash(self.source)``), so this key is only
        ever compared against another key built in the *same* process
        (``get_cached_deep_diagnostics``).  Do not persist it or send it across
        the process-pool boundary — use ``shared.hashing.stable_text_hash`` if a
        process-stable identity is ever needed there.
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
        self._replace_snapshot(
            deep_diag_proc_key=self.get_deep_diag_proc_key(),
            deep_diag_result=diagnostics,
        )

    def _segment_chunks(
        self,
        source: str,
        old_snap: _StateSnapshot,
        *,
        edit: "EditRange | None" = None,
        new_buffer: DocumentBuffer | None = None,
    ) -> list[TopLevelChunk]:
        """Segment *source* into top-level chunks, incrementally when possible.

        On an edit, the previous segmentation is reused everywhere the change
        did not reach (verbatim prefix + offset-shifted suffix), re-tokenising
        only the window straddling the edit.  The result is byte-identical to a
        from-scratch :func:`segment_top_level_chunks`; the incremental builder
        returns ``None`` (and we fall back to a full pass) whenever it cannot
        prove equivalence.

        *edit* / *new_buffer* let ``update_source_quick`` pass the edit range it
        already inferred and the rope it already spliced, so the incremental
        builder neither re-infers the edit nor builds a throwaway rope for its
        two offset→position conversions (it reuses the spliced rope, O(log n)).
        """
        if old_snap.chunks and old_snap.source:
            if edit is None:
                edit = infer_edit_range(old_snap.source, source)
            if edit is not None:
                # Reuse the spliced rope's position index when it matches the
                # new source; else the builder falls back to a direct scan.
                to_pos = (
                    new_buffer.offset_to_position
                    if new_buffer is not None and new_buffer.source == source
                    else None
                )
                inc = incremental_top_level_chunks(
                    old_snap.source, old_snap.chunks, source, edit, to_pos
                )
                if inc is not None:
                    return inc
        return segment_top_level_chunks(source)

    def update_source_quick(
        self,
        source: str,
        version: int | None = None,
    ) -> bool:
        self.refresh_dialect_hint(source)
        with self._signature_profile():
            return self._update_source_quick_for_active_profile(source, version)

    def _update_source_quick_for_active_profile(
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
        # Preserve the last fully-analysed snapshot so the later update() can
        # reuse it as the incremental base (this swap clears analysis).  Don't
        # overwrite it on a run of consecutive quick updates (old.analysis is
        # then already None) — keep the freshest *analysed* snapshot.
        if old.analysis is not None:
            self._prev_analysed = old
        # Reuse the prior rope: splice the edit into it in O(log n + |edit|)
        # instead of rebuilding the whole position index from scratch, so the
        # buffer that serves semantic-token requests in the window before full
        # analysis doesn't re-scan the document on every keystroke.  Falls back
        # to a fresh build (buffer=None → lazy from_source) when the previous
        # rope is unavailable or the edit can't be inferred.
        # Infer the edit once here: the rope splice below and the incremental
        # chunk segmentation both consume it (and its line_delta), instead of
        # each re-inferring the edit and recounting newlines.
        new_buffer: DocumentBuffer | None = None
        edit: EditRange | None = None
        if old.buffer is not None and old.buffer.source == old.source:
            edit = infer_edit_range(old.source, source)
            if edit is not None:
                new_buffer = DocumentBuffer.from_edit(
                    old.buffer,
                    source,
                    RopeEdit(edit.start, edit.old_end, edit.new_end, edit.line_delta),
                    version,
                )
        new_chunks = self._segment_chunks(source, old, edit=edit, new_buffer=new_buffer)
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
        self._swap_snapshot(
            _StateSnapshot(
                source=source,
                version=version,
                chunks=new_chunks,
                has_partial_commands=has_partial,
                chunk_caches=new_caches,
                file_profiles=old.file_profiles,
                buffer=new_buffer,
            )
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
        self.refresh_dialect_hint(source)
        # Compare against the dialect the *cached analysis* was built under, not
        # against the live dialect_hint: update_source_quick refreshes the hint
        # eagerly, so a same-length source-level dialect switch would otherwise
        # look unchanged here and reuse stale analysis.
        if self.dialect_hint != self._analysed_dialect:
            # A dialect change invalidates *all* cached analysis: chunk hashes
            # are position+text only and the per-proc/interproc cache keys omit
            # the dialect, yet a FunctionUnit's CFG/analysis depends on the
            # dialect's command set (e.g. ``try``/``lassign`` arg roles).  Force
            # a full rebuild so the new dialect is applied document-wide.
            force_reanalyse = True
        if force_reanalyse:
            # A hard refresh must not reuse per-proc/interproc units or the
            # preserved analysed snapshot: their keys (body text + position +
            # stub/context fingerprints) do not encode the dialect, so a dialect
            # switch would otherwise reuse a unit analysed under the old dialect.
            self._proc_cache = {}
            self._reposition_cache = {}
            self._interproc_cache = {}
            self._prev_analysed = None
        with self._signature_profile():
            self._update_for_active_profile(
                source,
                version,
                force_reanalyse=force_reanalyse,
                line_length=line_length,
            )
        # Record the dialect the now-current analysis was built under.
        if self.analysis is not None:
            self._analysed_dialect = self.dialect_hint

    def _update_for_active_profile(
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
        # Incremental base: the live snapshot when it is analysed, else the
        # snapshot update_source_quick() preserved before clearing analysis.
        # Using it means a quick update (new source, analysis=None) does not
        # force the next analysis down the cold full-rebuild path.
        base = old_snap if old_snap.analysis is not None else self._prev_analysed
        # Reuse chunks from update_source_quick() when the source matches,
        # avoiding a redundant O(source_len) lexer pass.
        if source == old_snap.source and old_snap.chunks:
            new_chunks = old_snap.chunks
        else:
            new_chunks = self._segment_chunks(source, old_snap)
        t_seg = time.perf_counter()
        has_partial = any(cmd.is_partial for chunk in new_chunks for cmd in chunk.commands)

        if not force_reanalyse and base is not None and base.analysis is not None:
            if source == base.source and base is old_snap:
                # Live snapshot is already the analysed result for this source.
                log.debug(
                    "[timing] document update %.0fms (unchanged)",
                    (time.perf_counter() - t0) * 1000,
                )
                return
            dirty_idx = find_first_dirty_chunk(base.chunks, new_chunks)
            if dirty_idx >= len(new_chunks) and dirty_idx >= len(base.chunks):
                # All chunks match the analysed base — reuse its analysis under
                # the new version (restores analysis a quick update cleared).
                new_snap = _StateSnapshot(
                    source=source,
                    version=version,
                    _tokens=base._tokens if source == base.source else None,
                    analysis=base.analysis,
                    compilation_unit=base.compilation_unit,
                    chunks=new_chunks,
                    has_partial_commands=has_partial,
                    file_profiles=base.file_profiles,
                    chunk_caches=base.chunk_caches if base.chunks == new_chunks else [],
                    buffer=self._carry_or_build_buffer(source, version),
                    deep_diag_proc_key=base.deep_diag_proc_key,
                    deep_diag_result=base.deep_diag_result,
                    proc_diag_cache=base.proc_diag_cache,
                )
                self._swap_snapshot(new_snap)
                log.debug(
                    "[timing] document update %.0fms (all chunks match)",
                    (time.perf_counter() - t0) * 1000,
                )
                return

            # Incremental path: try to reuse cached artefacts from the base.
            try:
                self._update_incremental(
                    source,
                    version,
                    base,
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
        # BIG-IP configuration files are not Tcl source — skip the
        # general Tcl analyser entirely.  Their dialect hint is
        # resolved to ``"f5-bigip"`` by `infer_document_dialect`.
        if self.dialect_hint == "f5-bigip":
            t0 = time.perf_counter()
            self._swap_snapshot(
                _StateSnapshot(
                    source=source,
                    version=version,
                    analysis=None,
                    compilation_unit=None,
                    chunks=new_chunks,
                    has_partial_commands=has_partial,
                    file_profiles=frozenset(),
                    chunk_caches=[],
                    buffer=self._carry_or_build_buffer(source, version),
                )
            )
            log.info(
                "[timing] _update_incremental %.0fms (bigip config — skipped Tcl analysis)",
                (time.perf_counter() - t0) * 1000,
            )
            return

        t0 = time.perf_counter()
        file_profiles = (
            EVENT_REGISTRY.compute_file_profiles(source) if is_irules_dialect() else frozenset()
        )
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
        prev_reposition_cache = dict(self._reposition_cache)
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
                reposition_cache=self._reposition_cache,
                interproc_cache=self._interproc_cache,
                prune_interproc_cache=not has_partial,
            )
        except Exception:
            t_lower = time.perf_counter()
            log.debug("document_state: incremental compilation failed", exc_info=True)
            self._proc_cache = prev_proc_cache
            self._reposition_cache = prev_reposition_cache
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

        analyser_disabled = _effective_disabled_diagnostics(self.uri)
        if restore_snapshot is not None:
            analyser = Analyser(disabled_diagnostics=analyser_disabled)
            analyser.restore(restore_snapshot)
            analysis, dirty_snapshots = analyser.analyse_chunked(
                source,
                dirty_chunk_commands,
                cu=compilation_unit,
                skip_stubs=True,
                file_path=self.uri,
            )
        else:
            # No snapshot to restore from — full chunked analysis.
            analyser = Analyser(disabled_diagnostics=analyser_disabled)
            analysis, dirty_snapshots = analyser.analyse_chunked(
                source,
                dirty_chunk_commands,
                cu=compilation_unit,
                file_path=self.uri,
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
        self._swap_snapshot(
            _StateSnapshot(
                source=source,
                version=version,
                analysis=analysis,
                compilation_unit=compilation_unit,
                chunks=new_chunks,
                has_partial_commands=has_partial,
                file_profiles=file_profiles,
                chunk_caches=new_chunk_caches,
                buffer=self._carry_or_build_buffer(source, version),
            )
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
            from bisect import bisect_right

            buf = DocumentBuffer.from_source(source)
            all_style_diags = _get_style_diag_all_fn()(source, line_length=line_length)
            diag_lines = [d.range.start.line for d in all_style_diags]
            # Seed the dedup cursor past diagnostics already owned by the clean
            # chunks [0:dirty_idx] (whose cached style diagnostics we keep), so a
            # boundary-line diagnostic shared with the last clean chunk is not
            # re-emitted by the first dirty chunk.
            style_consumed = (
                bisect_right(diag_lines, _chunk_line_range(buf, chunks[dirty_idx - 1])[2])
                if dirty_idx > 0
                else 0
            )

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
                    from compiler.lowering import lower_commands_to_ir

                    ir_stmts, ir_procs = lower_commands_to_ir(source, list(chunk.commands))

                # Partition pre-computed style diagnostics for this chunk.
                start_line, _sc, end_line, _ec = _chunk_line_range(buf, chunk)
                style_diags, style_consumed = _owned_style_diags(
                    all_style_diags, diag_lines, start_line, end_line, style_consumed
                )

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
                # Tokenizer flags must match the full-rebuild path
                # (_build_chunk_caches_standalone) exactly, or incrementally
                # precomputed tokens for a dirty chunk would differ from a full
                # reanalysis — e.g. an iRules file identified by dialect rather
                # than the .irul/.irule extension, or a .conf/.apl file.
                is_irules = (
                    uri_lower.endswith(".irul")
                    or uri_lower.endswith(".irule")
                    or is_irules_dialect()
                )
                chunk_toks = _get_precompute_chunk_tokens_fn()(
                    source,
                    chunk_line_ranges,
                    analysis=analysis,
                    is_irules=is_irules,
                    is_bigip_conf=uri_lower.endswith(".conf"),
                    is_apl=uri_lower.endswith(".apl"),
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
        # BIG-IP configuration files are not Tcl source — skip the
        # general Tcl analyser entirely.  Their dialect hint is
        # resolved to ``"f5-bigip"`` by `infer_document_dialect`.
        if self.dialect_hint == "f5-bigip":
            t0 = time.perf_counter()
            self._swap_snapshot(
                _StateSnapshot(
                    source=source,
                    version=version,
                    analysis=None,
                    compilation_unit=None,
                    chunks=new_chunks,
                    has_partial_commands=has_partial,
                    file_profiles=frozenset(),
                    chunk_caches=[],
                    buffer=self._carry_or_build_buffer(source, version),
                )
            )
            log.info(
                "[timing] _update_full %.0fms (bigip config — skipped Tcl analysis)",
                (time.perf_counter() - t0) * 1000,
            )
            return

        t0 = time.perf_counter()
        file_profiles = (
            EVENT_REGISTRY.compute_file_profiles(source) if is_irules_dialect() else frozenset()
        )
        t_tok = time.perf_counter()

        # Conf-wrapped iRules: extract rule bodies and analyse each
        # independently, then merge.  Skips compilation and chunk caching
        # since the outer structure is not Tcl.
        if is_irules_dialect():
            from dialects.f5.bigip.rule_extract import is_conf_wrapped_irules

            if is_conf_wrapped_irules(source):
                self._update_full_conf_wrapped(
                    source,
                    version,
                    new_chunks,
                    has_partial,
                    file_profiles=file_profiles,
                    line_length=line_length,
                )
                t_cw = time.perf_counter()
                log.info(
                    "[timing] _update_full (conf-wrapped) %.0fms",
                    (t_cw - t0) * 1000,
                )
                return

        prev_proc_cache = dict(self._proc_cache)
        prev_reposition_cache = dict(self._reposition_cache)
        prev_interproc_cache = dict(self._interproc_cache)
        compilation_unit: CompilationUnit | None = None
        try:
            compilation_unit = compile_source(
                source,
                proc_cache=self._proc_cache,
                reposition_cache=self._reposition_cache,
                interproc_cache=self._interproc_cache,
                prune_interproc_cache=not has_partial,
            )
        except Exception:
            log.debug("document_state: compilation failed, preserving caches", exc_info=True)
            self._proc_cache = prev_proc_cache
            self._reposition_cache = prev_reposition_cache
            self._interproc_cache = prev_interproc_cache
        t_compile = time.perf_counter()

        self._do_update_proc_cache(compilation_unit, has_partial)

        # Analyse and build chunk caches in a single pass: process commands
        # chunk-by-chunk, capturing snapshots at each boundary.  This avoids
        # the old pattern of running analyse() then re-analysing per-chunk.
        chunk_commands = [list(chunk.commands) for chunk in new_chunks]
        analyser = Analyser(disabled_diagnostics=_effective_disabled_diagnostics(self.uri))
        analysis, chunk_snapshots = analyser.analyse_chunked(
            source,
            chunk_commands,
            cu=compilation_unit,
            file_path=self.uri,
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
        self._swap_snapshot(
            _StateSnapshot(
                source=source,
                version=version,
                analysis=analysis,
                compilation_unit=compilation_unit,
                chunks=new_chunks,
                has_partial_commands=has_partial,
                file_profiles=file_profiles,
                chunk_caches=chunk_caches,
                buffer=self._carry_or_build_buffer(source, version),
            )
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

    def _update_full_conf_wrapped(
        self,
        source: str,
        version: int | None,
        new_chunks: list[TopLevelChunk],
        has_partial: bool,
        *,
        file_profiles: frozenset[str] = frozenset(),
        line_length: int = 120,
    ) -> None:
        """Full rebuild for conf-wrapped iRules files.

        Extracts each ``ltm rule`` / ``gtm rule`` body, analyses it as
        a standalone iRule, and merges the results with shifted ranges.
        """
        from analyser.conf_wrapped import analyse_conf_wrapped

        analysis, embedded_rules = analyse_conf_wrapped(
            source,
            disabled_diagnostics=_effective_disabled_diagnostics(self.uri),
            file_path=self.uri,
        )

        # Compute file profiles from ALL embedded rule bodies.
        all_profiles: set[str] = set()
        for rule in embedded_rules:
            all_profiles.update(EVENT_REGISTRY.compute_file_profiles(rule.body))
        file_profiles = frozenset(all_profiles) | file_profiles

        buf = DocumentBuffer.from_source(source, version)

        self._swap_snapshot(
            _StateSnapshot(
                source=source,
                version=version,
                analysis=analysis,
                compilation_unit=None,
                chunks=new_chunks,
                has_partial_commands=has_partial,
                file_profiles=file_profiles,
                chunk_caches=[],
                buffer=buf,
                conf_wrapped=True,
                embedded_rules=embedded_rules,
            )
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
            buf = DocumentBuffer.from_source(source)
            caches: list[ChunkCache | None] = []

            # Compute style diagnostics once for the whole file, then partition
            # by chunk via a monotonic cursor (each line-level diagnostic owned
            # by exactly one chunk, even when chunks share a physical line).
            all_style_diags = _get_style_diag_all_fn()(source, line_length=line_length)
            diag_lines = [d.range.start.line for d in all_style_diags]
            style_consumed = 0

            # Semantic tokens are pre-computed per chunk later via
            # ``precompute_chunk_tokens()``; we avoid an additional full-file
            # precompute here to prevent duplicate work.

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
                    from compiler.lowering import lower_commands_to_ir

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
                style_diags, style_consumed = _owned_style_diags(
                    all_style_diags, diag_lines, start_line, end_line, style_consumed
                )

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
                time.sleep(0)  # Yield GIL between chunks
            # Pre-compute semantic tokens per chunk so the first
            # semanticTokens/full request gets a full cache hit.
            try:
                chunk_line_ranges = [_chunk_line_range(buf, c) for c in chunks]
                uri_lower = self.uri.lower()
                # Tokenizer flags must match the subprocess full-rebuild path
                # (_build_chunk_caches_standalone) so incrementally/in-thread
                # precomputed tokens equal a from-scratch reanalysis — covers
                # iRules-by-dialect and .conf/.apl files.
                is_irules = (
                    uri_lower.endswith(".irul")
                    or uri_lower.endswith(".irule")
                    or is_irules_dialect()
                )
                chunk_toks = _get_precompute_chunk_tokens_fn()(
                    source,
                    chunk_line_ranges,
                    analysis=analysis,
                    is_irules=is_irules,
                    is_bigip_conf=uri_lower.endswith(".conf"),
                    is_apl=uri_lower.endswith(".apl"),
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
            # One IR pass builds both caches, sharing the expensive fingerprint /
            # CFG-context precomputation rather than walking the IR twice.
            next_proc_cache, next_reposition_cache = _build_proc_caches(compilation_unit)
            if has_partial:
                merged = dict(self._proc_cache)
                merged.update(next_proc_cache)
                self._proc_cache = merged
                merged_repos = dict(self._reposition_cache)
                merged_repos.update(next_reposition_cache)
                self._reposition_cache = merged_repos
            else:
                self._proc_cache = next_proc_cache
                self._reposition_cache = next_reposition_cache
        else:
            if not has_partial:
                self._proc_cache = {}
                self._reposition_cache = {}
                self._interproc_cache = {}


class WorkspaceState:
    """Manages DocumentState objects for all open documents."""

    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._documents: dict[str, DocumentState] = {}

    def get(self, uri: str) -> DocumentState | None:
        with self._lock:
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
        state = DocumentState(
            uri=uri,
            language_id=language_id,
            dialect_hint=infer_document_dialect(uri, source, language_id),
        )
        if analyse:
            state.update(source, version, force_reanalyse=force_reanalyse, line_length=line_length)
        else:
            # Lightweight open: store source, version, and chunks without
            # running the analysis pipeline.  Segmenting chunks and building
            # a DocumentBuffer here (~30ms for large files) enables eager
            # semantic-token precomputation in _publish_diagnostics so the
            # editor gets syntax highlighting before the heavy analysis.
            chunks = segment_top_level_chunks(source)
            has_partial = any(cmd.is_partial for chunk in chunks for cmd in chunk.commands)
            state._swap_snapshot(
                _StateSnapshot(
                    source=source,
                    version=version,
                    chunks=chunks,
                    has_partial_commands=has_partial,
                    buffer=DocumentBuffer.from_source(source, version),
                )
            )
        with self._lock:
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
        with self._lock:
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
        with self._lock:
            self._documents.pop(uri, None)

    def get_language_id(self, uri: str) -> str:
        """Return the ``language_id`` from the editor for *uri*, or ``""``."""
        with self._lock:
            state = self._documents.get(uri)
        return state.language_id if state is not None else ""

    def items(self) -> list[tuple[str, DocumentState]]:
        """Return all open documents."""
        with self._lock:
            return list(self._documents.items())

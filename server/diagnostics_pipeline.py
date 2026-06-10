"""Diagnostic publishing pipeline: push/pull cache, async analysis, dialect helpers."""

from __future__ import annotations

import asyncio
import functools
import itertools
import logging
import time
from concurrent.futures.process import BrokenProcessPool
from typing import TYPE_CHECKING

from lsprotocol import types

import server.state as _state

from .features.diagnostics import (
    _to_lsp_diagnostic,
    get_basic_diagnostics,
    get_deep_diagnostics,
    get_diagnostics,
)
from .workspace.scanner import is_bigip_conf_name, path_to_uri, uri_to_path
from .workspace.workspace_index import EntrySource

if TYPE_CHECKING:
    from pygls.lsp.server import LanguageServer

log = logging.getLogger(__name__)

# Server reference

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    global _server
    _server = server_instance


def _require_server() -> LanguageServer:
    if _server is None:
        raise RuntimeError("lsp.diagnostics_pipeline: server not configured")
    return _server


# Pull diagnostics cache

_pull_diag_cache: dict[str, list[types.Diagnostic]] = {}
_pull_diag_result_ids: dict[str, str] = {}
_pull_diag_counter = itertools.count(1)


def _next_pull_diag_result_id() -> str:
    return f"tcl-lsp-diag-{next(_pull_diag_counter)}"


# Per-document single-writer serialization (W1)
#
# Guarantees at most one in-flight analysis+publish per document — the
# single-writer invariant the persistent incremental graph will rely on, so
# two overlapping did_change coroutines can never mutate a document's state
# concurrently. Distinct uris get distinct locks, so documents still analyse
# in parallel. ``_publish_latest_version`` lets a request that was superseded
# while queued bail *before* the expensive analysis (avoids a rapid-typing
# pileup that a bare lock would create).
_publish_locks: dict[str, asyncio.Lock] = {}
_publish_latest_version: dict[str, int] = {}


def _get_publish_lock(uri: str) -> asyncio.Lock:
    lock = _publish_locks.get(uri)
    if lock is None:
        lock = asyncio.Lock()
        _publish_locks[uri] = lock
    return lock


def _superseded(uri: str, version: int | None) -> bool:
    """True when a newer version was requested while this run was analysing.

    The pool ``await`` is a suspension point: a later ``did_change`` runs the
    top of ``_publish_diagnostics`` (bumping ``_publish_latest_version``) before
    queueing on the writer lock.  A stale result must therefore re-check this
    *after* analysis — before swapping state or publishing — or it would clobber
    the document with an old analysis and publish out-of-date diagnostics.
    """
    if version is None:
        return False
    latest = _publish_latest_version.get(uri)
    return latest is not None and version < latest


def _release_publish_state(uri: str) -> None:
    """Drop a document's per-URI state (call on did_close).

    Includes the pull-model diagnostics cache: without this, every ever-opened
    URI kept a full diagnostics list (and its result id) resident for the life
    of the session — a slow, unbounded leak across a long editing session that
    opens many files.
    """
    _publish_locks.pop(uri, None)
    _publish_latest_version.pop(uri, None)
    _pull_diag_cache.pop(uri, None)
    _pull_diag_result_ids.pop(uri, None)


# asyncio keeps only a *weak* reference to the result of ``create_task`` /
# ``ensure_future``, so a fire-and-forget analysis task can be garbage-collected
# mid-flight — silently dropping a document's diagnostics (a documented asyncio
# foot-gun).  Hold a strong reference until the task completes.
_background_tasks: set[asyncio.Task] = set()


def spawn_publish_diagnostics(
    uri: str,
    source: str,
    version: int | None = None,
    *,
    force_reanalyse: bool = False,
    loop: asyncio.AbstractEventLoop | None = None,
) -> asyncio.Task:
    """Fire-and-forget ``_publish_diagnostics`` while holding a strong reference.

    The three callers that schedule a publish without awaiting it (did_open, the
    reanalyse-all command, and a config change) all route through here so the
    task can't be collected before it finishes.  Pass *loop* when scheduling from
    a sync context that already resolved the running loop; otherwise the current
    running loop is used.
    """
    coro = _publish_diagnostics(uri, source, version, force_reanalyse=force_reanalyse)
    task = loop.create_task(coro) if loop is not None else asyncio.ensure_future(coro)
    _background_tasks.add(task)
    task.add_done_callback(_background_tasks.discard)
    return task


# Fresh-build routing by source size.  Every *fresh* build (full, non-
# incremental analysis) runs in a subprocess pool — CPU-bound analysis is
# GIL-bound, so in-thread fan-out is slower than serial (measured 4-thread
# 20.8s vs serial 15.3s vs 4-process 7.7s).  A build at/below this size uses the
# small-file lane (:func:`server.state._get_small_pool`); a larger one uses the
# cold pool.  The two lanes are separate pools so a trivial file never queues
# behind a workspace of multi-second cold builds (head-of-line blocking).
_SMALL_BUILD_MAX_BYTES = 16 * 1024
# A generous wall-clock ceiling for a *big* cold build in the pool, far above any
# legitimate build (per-proc cost is bounded by the complexity guard).  Unlike
# the old per-build timeout it does NOT re-run the build in-thread on expiry
# (that doubled the work and froze the event loop); it poisons-and-recreates the
# cold pool to reclaim a wedged worker and abandons the build, releasing the
# per-uri writer lock so later edits to that document aren't starved.
_COLD_BUILD_CEILING_S = 120.0
# The small-file lane's ceiling.  A genuinely small build finishes in well under
# a second; this is a large margin that still reclaims a wedged small worker far
# faster than the cold ceiling.
_SMALL_BUILD_CEILING_S = 30.0


def _publish_diags_to_client(
    uri: str,
    diagnostics: list[types.Diagnostic],
    version: int | None = None,
) -> None:
    """Push a diagnostics notification to the client and update the pull cache."""
    _pull_diag_cache[uri] = list(diagnostics)
    _pull_diag_result_ids[uri] = _next_pull_diag_result_id()
    if _server is None:
        log.error(
            "_publish_diags_to_client called before configure(); dropping publish for %s", uri
        )
        return
    _server.text_document_publish_diagnostics(
        types.PublishDiagnosticsParams(
            uri=uri,
            diagnostics=diagnostics,
            version=version,
        )
    )


# Pull-model handlers


def on_document_diagnostic(
    params: types.DocumentDiagnosticParams,
) -> types.RelatedFullDocumentDiagnosticReport | types.RelatedUnchangedDocumentDiagnosticReport:
    uri = params.text_document.uri
    cached = _pull_diag_cache.get(uri, [])
    current_result_id = _pull_diag_result_ids.get(uri)
    previous = getattr(params, "previous_result_id", None)
    if current_result_id is not None and previous is not None and previous == current_result_id:
        return types.RelatedUnchangedDocumentDiagnosticReport(
            result_id=current_result_id,
        )
    return types.RelatedFullDocumentDiagnosticReport(
        items=list(cached),
        result_id=current_result_id,
    )


def on_workspace_diagnostic(
    params: types.WorkspaceDiagnosticParams,
) -> types.WorkspaceDiagnosticReport:
    previous_ids: dict[str, str] = {}
    for item in getattr(params, "previous_result_ids", []) or []:
        previous_ids[item.uri] = item.value

    report_items: list[
        types.WorkspaceFullDocumentDiagnosticReport
        | types.WorkspaceUnchangedDocumentDiagnosticReport
    ] = []
    for uri, diagnostics in _pull_diag_cache.items():
        current_result_id = _pull_diag_result_ids.get(uri, "")
        prev = previous_ids.get(uri)
        if prev is not None and prev == current_result_id and current_result_id:
            report_items.append(
                types.WorkspaceUnchangedDocumentDiagnosticReport(
                    uri=uri,
                    result_id=current_result_id,
                    version=None,
                )
            )
        else:
            report_items.append(
                types.WorkspaceFullDocumentDiagnosticReport(
                    uri=uri,
                    items=list(diagnostics),
                    result_id=current_result_id or None,
                    version=None,
                )
            )
    return types.WorkspaceDiagnosticReport(items=report_items)


# Workspace diagnostic context


def _build_workspace_diagnostic_context():
    from analyser.semantic_model import WorkspaceDiagnosticContext

    ws_pkg_names: set[str] = set()
    pkg_by_uri: dict[str, frozenset[str]] = {}
    alias_by_uri: dict[str, frozenset[str]] = {}
    for pkg_uri in _state.workspace_index.all_uris():
        analysis = _state.workspace_index.get_analysis(pkg_uri)
        if analysis is not None:
            names = analysis.active_package_names()
            ws_pkg_names.update(names)
            if names:
                pkg_by_uri[pkg_uri] = names
            if analysis.command_aliases:
                tails = frozenset(qn.rsplit("::", 1)[-1] for qn in analysis.command_aliases if qn)
                if tails:
                    alias_by_uri[pkg_uri] = tails

    return WorkspaceDiagnosticContext(
        workspace_proc_names=frozenset(_state.workspace_index.all_proc_names()),
        workspace_package_names=frozenset(ws_pkg_names),
        package_names_by_uri=pkg_by_uri,
        source_graph=_state.workspace_index.source_graph_snapshot(),
        alias_names_by_uri=alias_by_uri,
    )


# Workspace index update


def _update_workspace_index(uri: str, source: str, state: object) -> None:
    from .workspace.document_state import DocumentState

    if not isinstance(state, DocumentState):
        return
    if not state.analysis or state.has_partial_commands:
        return
    _state.workspace_index.update(uri, state.analysis, EntrySource.OPEN)
    if state.analysis.source_targets:
        from analyser.source_resolver import resolve_source_target

        script_path = uri_to_path(uri) or ""
        ws_roots = _state.background_scanner.workspace_roots
        sourced: set[str] = set()
        for st in state.analysis.source_targets:
            resolved = resolve_source_target(st.raw_path, st.is_literal, script_path, ws_roots)
            if resolved:
                sourced.add(path_to_uri(resolved))
        _state.workspace_index.update_source_graph(uri, frozenset(sourced))
    if _is_irules_source(uri) or state.conf_wrapped:
        if state.analysis.all_procs:
            _state.workspace_index.update_irules_globals(
                uri,
                state.analysis.all_procs,
            )
        from compiler.irules_flow import extract_rule_init_vars

        if state.conf_wrapped and state.embedded_rules:
            all_exports: list = []
            for rule in state.embedded_rules:
                all_exports.extend(extract_rule_init_vars(rule.body))
            _state.workspace_index.update_rule_init_vars(uri, all_exports)
        else:
            exports = extract_rule_init_vars(source, cu=state.compilation_unit)
            _state.workspace_index.update_rule_init_vars(uri, exports)
    _load_packages_if_needed(state.analysis, uri)


# Synchronous publish (used during config changes)


def _publish_diagnostics_sync(
    uri: str,
    source: str,
    version: int | None = None,
    *,
    force_reanalyse: bool = False,
) -> None:
    from analyser.checks._style import non_ascii_mode_scope
    from compiler.registry.dialect import dialect_scope
    from compiler.registry.stub_comments import ambient_stub_scope

    cfg = _state.config_for_uri(uri)
    dialect, extras = _state.resolve_dialect_for_uri(uri, source)
    # BIG-IP configuration files have their own diagnostics pipeline
    # (bigip parser → bigip validator).  The general Tcl analyser
    # must never be run on their top-level text.
    if dialect == "f5-bigip":
        return
    with (
        dialect_scope(dialect=dialect, extra_commands=extras),
        non_ascii_mode_scope(cfg.non_ascii_mode),
        ambient_stub_scope(_state.workspace_stub_commands),
    ):
        state = _state.workspace_state.update(
            uri,
            source,
            version,
            force_reanalyse=force_reanalyse,
            line_length=cfg.line_length,
        )
        partial_mode = state.has_partial_commands

        if cfg.diagnostics_enabled:
            ws_ctx = _build_workspace_diagnostic_context()
            diagnostics = get_diagnostics(
                source,
                analysis=state.analysis,
                cu=state.compilation_unit,
                optimiser_enabled=cfg.optimiser_enabled and not partial_mode,
                shimmer_enabled=cfg.shimmer_enabled and not partial_mode,
                taint_enabled=not partial_mode,
                xc_diagnostics_enabled=cfg.xc_diagnostics_enabled,
                disabled_diagnostics=cfg.disabled_diagnostics,
                disabled_optimisations=cfg.disabled_optimisations,
                uri=uri,
                line_length=cfg.line_length,
                workspace_context=ws_ctx,
            )
        else:
            diagnostics = []

        _publish_diags_to_client(uri, diagnostics, version)
        _update_workspace_index(uri, source, state)


# Async publish pipeline


async def _publish_diagnostics(
    uri: str,
    source: str,
    version: int | None = None,
    *,
    force_reanalyse: bool = False,
) -> None:
    # Record the newest requested version before queueing on the writer lock,
    # so an older request that loses the race can detect it was superseded.
    if version is not None:
        prev = _publish_latest_version.get(uri)
        if prev is None or version >= prev:
            _publish_latest_version[uri] = version

    async with _get_publish_lock(uri):
        # Superseded while we waited for the lock: a newer version is already
        # queued/running and will publish the authoritative result. Bail
        # before doing the expensive analysis. (force_reanalyse passes
        # version=None and is never superseded.)
        if version is not None:
            latest = _publish_latest_version.get(uri)
            if latest is not None and version < latest:
                log.info(
                    "[timing] _publish_diagnostics superseded before analysis "
                    "(newest req v%s, this v%s)",
                    latest,
                    version,
                )
                return
        await _publish_diagnostics_locked(uri, source, version, force_reanalyse=force_reanalyse)


async def _publish_diagnostics_locked(
    uri: str,
    source: str,
    version: int | None = None,
    *,
    force_reanalyse: bool = False,
) -> None:
    from analyser.checks._style import non_ascii_mode_scope
    from compiler.registry.dialect import dialect_scope
    from compiler.registry.stub_comments import ambient_stub_scope

    dialect, extras = _state.resolve_dialect_for_uri(uri, source)
    cfg = _state.config_for_uri(uri)
    # Resolve once at the top so every step of this coroutine — quick
    # parsing, semantic-token precompute, subprocess-pool analysis, the
    # deep-diagnostics worker, and the final publish — operates under the
    # same per-folder dialect / non-ASCII mode (issue #407).  ContextVar
    # values propagate to ``asyncio.to_thread`` / pool workers via
    # ``contextvars.copy_context`` which ``run_in_executor`` and
    # ``to_thread`` use internally; the subprocess pool worker
    # (_analyse_document_fresh) gets the dialect via its explicit
    # ``dialect=`` argument.
    with (
        dialect_scope(dialect=dialect, extra_commands=extras),
        non_ascii_mode_scope(cfg.non_ascii_mode),
        ambient_stub_scope(_state.workspace_stub_commands),
    ):
        await _publish_diagnostics_inner(uri, source, version, force_reanalyse=force_reanalyse)


async def _publish_diagnostics_inner(
    uri: str,
    source: str,
    version: int | None = None,
    *,
    force_reanalyse: bool = False,
) -> None:
    # BIG-IP configuration files have their own diagnostics pipeline
    # (bigip parser → bigip validator).  The general Tcl analyser must
    # never be run on their top-level text.
    if _is_bigip_conf(uri):
        return

    t_start = time.perf_counter()

    await asyncio.sleep(0)

    t_quick = time.perf_counter()
    state = _state.workspace_state.get(uri)
    if state is None:
        state = _state.workspace_state.open(uri, source, version, analyse=False)
        needs_analysis = True
    else:
        needs_analysis = state.update_source_quick(source, version)
        if not needs_analysis and state.analysis is None:
            needs_analysis = True
    quick_ms = (time.perf_counter() - t_quick) * 1000

    if state.get_semantic_token_cache() is None and state.chunks:
        is_cw_pre = state.conf_wrapped
        state.precompute_syntax_tokens(
            is_irules=_is_irules_source(uri) or is_cw_pre,
            is_bigip_conf=_is_bigip_conf(uri) or is_cw_pre,
            is_apl=_is_apl_source(uri),
        )

    await asyncio.sleep(0)

    cfg = _state.config_for_uri(uri)
    disabled_diagnostics = set(cfg.disabled_diagnostics)
    line_length = cfg.line_length
    optimiser_enabled = cfg.optimiser_enabled
    disabled_optimisations = set(cfg.disabled_optimisations)

    t_update = time.perf_counter()
    did_analyse = needs_analysis or force_reanalyse
    subprocess_result: dict | None = None
    if did_analyse:
        # "Fresh" = full (non-incremental) build.  A warm edit clears analysis
        # via update_source_quick but preserves an incremental base
        # (``can_analyse_incrementally``), so it takes the in-thread incremental
        # update() path — cheap, and it mutates the DocumentState in place.
        #
        # Every *fresh* build runs in a subprocess pool: CPU-bound analysis is
        # GIL-bound, so in-thread fan-out is slower than serial under a storm.
        # A large build uses the cold pool; a small one uses a separate
        # small-file lane (its own pool) so a trivial file never queues behind a
        # workspace of multi-second cold builds (head-of-line blocking).
        is_fresh = force_reanalyse or not state.can_analyse_incrementally
        if is_fresh:
            from analyser.checks._style import _non_ascii_mode_var
            from compiler.dialect_context import _dialect_var
            from compiler.registry.runtime import _extra_commands_var
            from server.workspace.document_state import _analyse_document_fresh

            lane = "cold" if len(source) > _SMALL_BUILD_MAX_BYTES else "small"

            # Cheap pre-submission supersession check: if a newer edit was
            # already requested while this build queued behind the writer lock,
            # don't even acquire a pool worker for a result we'd discard at line
            # ~540 anyway.  Submitting it would occupy a (scarce) subprocess slot
            # the newer build needs, so the live edit queues behind a corpse.
            if _superseded(uri, version):
                log.info(
                    "[timing] _publish_diagnostics abandoned (superseded before "
                    "%s submit: newest req v%s, this v%s)",
                    lane,
                    _publish_latest_version.get(uri),
                    version,
                )
                return

            if lane == "cold":
                pool = _state._get_process_pool()
                reset_pool = _state._reset_process_pool
                ceiling = _COLD_BUILD_CEILING_S
            else:
                pool = _state._get_small_pool()
                reset_pool = _state._reset_small_pool
                ceiling = _SMALL_BUILD_CEILING_S

            # The build runs in its lane's pool and is abandoned if a newer edit
            # supersedes it.  A generous wall-clock ceiling guards a wedged build
            # the complexity guard doesn't catch: unlike the old per-build
            # timeout it does NOT re-run in-thread on expiry (that doubled the
            # work and froze the event loop) — it poisons-and-recreates the
            # lane's pool and returns, releasing this URI's writer lock so later
            # edits aren't starved.  A worker crash surfaces as BrokenProcessPool
            # below.
            try:
                loop = asyncio.get_running_loop()
                result = await asyncio.wait_for(
                    loop.run_in_executor(
                        pool,
                        functools.partial(
                            _analyse_document_fresh,
                            source=source,
                            version=version,
                            line_length=line_length,
                            dialect=_dialect_var.get(),
                            uri=uri,
                            disabled_diagnostics=disabled_diagnostics,
                            disabled_optimisations=disabled_optimisations,
                            optimiser_enabled=optimiser_enabled,
                            # Forward every per-request ContextVar value
                            # so the subprocess sees the per-folder
                            # dialect / extras / nonAscii (issue #407).
                            # Read the effective non-ASCII mode from the
                            # ContextVar rather than ``cfg.non_ascii_mode``
                            # so the subprocess inherits the workspace
                            # default when a folder hasn't overridden it.
                            extra_commands=_extra_commands_var.get(),
                            non_ascii_mode=_non_ascii_mode_var.get(),
                            # External .tcl.stubs are workspace state, not a
                            # ContextVar, so forward them explicitly for the
                            # subprocess to re-establish (mirrors dialect).
                            stub_commands=tuple(_state.workspace_stub_commands),
                            # Forward the line-ending config and the (small,
                            # picklable) workspace diagnostic context so the
                            # subprocess build's basic diagnostics match the
                            # in-thread phase1: line-ending checks + W120/W123
                            # workspace filtering.
                            line_ending=_state.formatter_config_for_uri(uri).line_ending,
                            workspace_context=_build_workspace_diagnostic_context(),
                        ),
                    ),
                    timeout=ceiling,
                )
                if _superseded(uri, version):
                    # A newer version was requested while we were analysing —
                    # don't overwrite the document with this stale result.
                    log.info(
                        "[timing] _publish_diagnostics abandoned (superseded after "
                        "analysis: newest req v%s, this v%s)",
                        _publish_latest_version.get(uri),
                        version,
                    )
                    return
                state.apply_subprocess_result(result, version)
                subprocess_result = result
            except asyncio.TimeoutError:
                # The build blew past the generous ceiling — almost certainly a
                # wedged worker.  Poison-and-recreate the lane's pool to reclaim
                # it and abandon this build; do NOT re-run in-thread (that would
                # block the event loop on the same wedge).  The orphaned pool
                # future is uncancellable but dies with the pool.
                log.warning(
                    "%s build exceeded %.0fs ceiling for %s (v%s); "
                    "recreating %s pool and abandoning",
                    lane,
                    ceiling,
                    uri,
                    version,
                    lane,
                )
                reset_pool()
                return
            except BrokenProcessPool:
                log.warning("%s build pool broken, falling back to thread", lane)
                reset_pool()
                await asyncio.to_thread(
                    state.update,
                    source,
                    version,
                    force_reanalyse=force_reanalyse,
                    line_length=line_length,
                )
            except Exception:
                log.warning("Subprocess analysis failed, falling back to thread", exc_info=True)
                await asyncio.to_thread(
                    state.update,
                    source,
                    version,
                    force_reanalyse=force_reanalyse,
                    line_length=line_length,
                )
        else:
            # Warm/incremental edit: cheap; runs in a worker thread (it mutates
            # the DocumentState in place, so it can't go to a subprocess).
            await asyncio.to_thread(
                state.update,
                source,
                version,
                force_reanalyse=force_reanalyse,
                line_length=line_length,
            )
    update_ms = (time.perf_counter() - t_update) * 1000
    log.info(
        "[timing] workspace_state.update %.0fms (quick=%.0fms, uri=%s, lines=%d)",
        update_ms,
        quick_ms,
        uri,
        len(state.buffer.line_starts),
    )

    if state.version != version or _superseded(uri, version):
        log.info(
            "[timing] _publish_diagnostics abandoned (stale: have v%s, want v%s, newest v%s)",
            state.version,
            version,
            _publish_latest_version.get(uri),
        )
        return

    partial_mode = state.has_partial_commands

    if did_analyse and state.analysis is not None:
        try:
            _require_server().workspace_semantic_tokens_refresh(None)
        except Exception:
            pass
        try:
            _require_server().workspace_folding_range_refresh(None)
        except Exception:
            pass

    if not cfg.diagnostics_enabled:
        basic_diags: list[types.Diagnostic] = []
        analysis_result = None
        suppressed: dict[int, frozenset[str]] = {}
    elif subprocess_result is not None and "basic_diags" in subprocess_result:
        basic_diags = subprocess_result["basic_diags"]
        analysis_result = subprocess_result.get("analysis")
        suppressed = subprocess_result.get("suppressed", {})
        log.info("[timing] phase1 diagnostics 0ms (from subprocess, diags=%d)", len(basic_diags))
    else:
        cached_style = state.get_cached_style_diagnostics(
            disabled_diagnostics=disabled_diagnostics,
            line_length=line_length,
        )

        ws_ctx = _build_workspace_diagnostic_context()
        formatter_cfg = _state.formatter_config_for_uri(uri)

        def _phase1():
            return get_basic_diagnostics(
                source,
                analysis=state.analysis,
                cu=state.compilation_unit,
                optimiser_enabled=optimiser_enabled and not partial_mode,
                disabled_diagnostics=disabled_diagnostics,
                disabled_optimisations=disabled_optimisations,
                line_length=line_length,
                line_ending=formatter_cfg.line_ending,
                cached_style_diagnostics=cached_style,
                workspace_context=ws_ctx,
                uri=uri,
            )

        t_phase1 = time.perf_counter()
        basic_diags, analysis_result, suppressed = await asyncio.to_thread(_phase1)
        phase1_ms = (time.perf_counter() - t_phase1) * 1000
        log.info("[timing] phase1 diagnostics %.0fms (diags=%d)", phase1_ms, len(basic_diags))

    if not cfg.diagnostics_enabled:
        _publish_diags_to_client(uri, [], version)
        _update_workspace_index(uri, source, state)
        return

    _publish_diags_to_client(uri, basic_diags, version)
    _update_workspace_index(uri, source, state)
    total_ms = (time.perf_counter() - t_start) * 1000
    log.info("[timing] _publish_diagnostics total %.0fms (basic diags published)", total_ms)

    if partial_mode:
        return

    if not did_analyse and state.analysis is None:
        return

    opt_enabled = cfg.optimiser_enabled and not partial_mode
    shimmer_enabled = cfg.shimmer_enabled and not partial_mode
    taint_enabled = not partial_mode
    xc_enabled = cfg.xc_diagnostics_enabled
    disabled_diags = set(cfg.disabled_diagnostics)
    disabled_opts = set(cfg.disabled_optimisations)
    cu = state.compilation_unit

    cached_deep = state.get_cached_deep_diagnostics()

    if cached_deep is not None:
        _publish_diags_to_client(uri, basic_diags + cached_deep, version)
        return

    _state_ref = state
    _scheduled_version = version

    from compiler.registry.dialect import active_dialect
    from compiler.registry.runtime import _extra_commands_var
    from server.features.diagnostics import _run_deep_diagnostics

    _dialect = active_dialect()
    # Snapshot the per-folder command overlay in the parent (the worker can't
    # read the parent's ContextVar) so the deep pass re-establishes it.
    _extra_commands = tuple(_extra_commands_var.get())
    # Deep diagnostics run in their OWN pool, separate from cold builds, so a
    # storm of multi-second cold builds can't starve every document's deep pass.
    _pool = _state._get_deep_pool()
    _generic_var_patterns = list(cfg.generic_variable_patterns)

    # Body-local (shimmer) diagnostic memoization (W2 leaf tier): recompute
    # shimmer only for procs whose body/context changed; reuse re-offset cached
    # shimmer for the clean rest.  ``_proc_infos`` is falsy (None / no procs)
    # when memoization can't apply (no CU, shimmer disabled, or procs can't be
    # positioned consistently) — then we run the full shimmer pass and skip the
    # cache.  ``_shimmer_targets`` is the dirty qname set passed to the pass.
    from server.features.incremental_diagnostics import (
        merge_memoized_deep,
        proc_diag_infos,
        split_clean_dirty,
    )

    _proc_infos = proc_diag_infos(cu) if (cu is not None and shimmer_enabled) else None
    _prev_proc_diag = state.get_proc_diag_cache() or {}
    _shimmer_targets: frozenset[str] | None = (
        split_clean_dirty(_proc_infos, _prev_proc_diag)[1] if _proc_infos else None
    )

    async def _deep_coro() -> list[types.Diagnostic]:
        # Cheap pre-submission supersession check: the deep coro is scheduled and
        # may sit behind earlier work before it runs.  If a newer edit already
        # arrived, don't burn a deep-pool worker on a result ``_guarded_publish``
        # will discard anyway — submitting it queues the live edit behind a corpse.
        if _superseded(uri, version):
            log.info(
                "[timing] deep build abandoned (superseded before submit: "
                "newest req v%s, this v%s)",
                _publish_latest_version.get(uri),
                version,
            )
            return []
        try:
            loop = asyncio.get_running_loop()
            result = await loop.run_in_executor(
                _pool,
                functools.partial(
                    _run_deep_diagnostics,
                    source=source,
                    suppressed=dict(suppressed),
                    dialect=_dialect,
                    optimiser_enabled=opt_enabled,
                    shimmer_enabled=shimmer_enabled,
                    taint_enabled=taint_enabled,
                    xc_diagnostics_enabled=xc_enabled,
                    disabled_diagnostics=disabled_diags,
                    disabled_optimisations=disabled_opts,
                    uri=uri,
                    generic_variable_patterns=_generic_var_patterns,
                    shimmer_target_procs=_shimmer_targets,
                    # The deep pool's workers don't share the parent's
                    # ContextVars, so forward the per-folder command overlay and
                    # workspace .tcl.stubs for the worker to re-establish — the
                    # same context the parent applies via dialect_scope(...,
                    # extra_commands) + ambient_stub_scope (issue #407).
                    extra_commands=_extra_commands,
                    stub_commands=tuple(_state.workspace_stub_commands),
                ),
            )
        except BrokenProcessPool:
            log.warning("Deep-diagnostics pool broken, falling back to thread")
            _state._deep_pool = None
            result = await asyncio.to_thread(
                get_deep_diagnostics,
                source,
                suppressed,
                cu=cu,
                analysis=analysis_result,
                optimiser_enabled=opt_enabled,
                shimmer_enabled=shimmer_enabled,
                taint_enabled=taint_enabled,
                xc_diagnostics_enabled=xc_enabled,
                disabled_diagnostics=disabled_diags,
                disabled_optimisations=disabled_opts,
                uri=uri,
                generic_variable_patterns=_generic_var_patterns,
                shimmer_target_procs=_shimmer_targets,
            )
        except Exception:
            log.warning("Subprocess deep diagnostics failed, falling back to thread", exc_info=True)
            result = await asyncio.to_thread(
                get_deep_diagnostics,
                source,
                suppressed,
                cu=cu,
                analysis=analysis_result,
                optimiser_enabled=opt_enabled,
                shimmer_enabled=shimmer_enabled,
                taint_enabled=taint_enabled,
                xc_diagnostics_enabled=xc_enabled,
                disabled_diagnostics=disabled_diags,
                disabled_optimisations=disabled_opts,
                uri=uri,
                generic_variable_patterns=_generic_var_patterns,
                shimmer_target_procs=_shimmer_targets,
            )
        # Merge the dirty-only shimmer recompute with the re-offset cache for
        # clean procs (non-body-local codes pass through unchanged).  Byte-for-
        # byte identical to a full deep pass — gated by test_incremental_diagnostics.
        new_proc_diag: dict | None = None
        if _proc_infos:
            result, new_proc_diag = merge_memoized_deep(_proc_infos, _prev_proc_diag, result)
        if _state_ref.version == _scheduled_version:
            _state_ref.store_deep_diagnostics(result)
            if new_proc_diag is not None:
                _state_ref.store_proc_diag_cache(new_proc_diag)
        return result

    def _guarded_publish(u: str, diags: list[types.Diagnostic], v: int | None) -> None:
        # The deep pass publishes after its background await; a newer edit may
        # have arrived meanwhile.  Re-check supersession before publishing (and
        # before overwriting the pull cache), matching the basic path's guards —
        # the scheduler's task cancellation alone races the publish.
        if _superseded(u, v):
            log.info(
                "[timing] deep publish abandoned (superseded: newest v%s, this v%s)",
                _publish_latest_version.get(u),
                v,
            )
            return
        _publish_diags_to_client(u, diags, v)

    _state.diagnostic_scheduler.schedule_async(
        uri,
        version,
        basic_diags,
        _deep_coro,
        _guarded_publish,
    )


# Auto-path safety check


def _is_unsafe_auto_path(path: str) -> bool:
    import os

    norm = os.path.abspath(path)
    if norm in ("/", os.path.sep) or norm == os.path.expanduser("~"):
        return True
    parts = [p for p in norm.split(os.sep) if p]
    return len(parts) < 2


# Package loading


def _load_packages_if_needed(analysis: object, uri: str | None = None) -> None:
    from analyser.semantic_model import AnalysisResult

    if not isinstance(analysis, AnalysisResult):
        return

    # Per-folder PackageResolver selection (issue #407): documents in
    # folders with their own ``tclLsp.libraryPaths`` use a dedicated
    # resolver configured with those paths; everything else falls back to
    # the workspace-level resolver.
    resolver = _state.package_resolver_for_uri(uri)

    if analysis.auto_path_entries and uri is not None:
        import os

        from analyser.auto_path_eval import evaluate_auto_path_expr

        file_path = uri_to_path(uri)
        extra_paths: list[str] = []
        for entry in analysis.auto_path_entries:
            resolved = evaluate_auto_path_expr(entry.raw, file_path)
            if not resolved or not os.path.isdir(resolved):
                continue
            if _is_unsafe_auto_path(resolved):
                log.warning(
                    "auto_path: refusing to scan overly broad directory %s (from %s)",
                    resolved,
                    file_path,
                )
                continue
            if resolved not in extra_paths:
                extra_paths.append(resolved)
        if extra_paths:
            resolver.add_search_paths(extra_paths)

    if not analysis.package_requires:
        return
    resolver_key = id(resolver)
    for pkg_req in analysis.package_requires:
        # Key the load-once cache by (resolver, name) so per-folder
        # resolvers (issue #407) can independently resolve the same
        # package name to different files — folder A's ``Foo`` from
        # /opt/tcllib doesn't block folder B's ``Foo`` from /usr/lib.
        if (resolver_key, pkg_req.name) in _state._loaded_packages:
            continue
        source_files = resolver.resolve(pkg_req.name, pkg_req.version)
        if not source_files:
            continue
        _state._loaded_packages.add((resolver_key, pkg_req.name))
        for file_path in source_files:
            pkg_uri = path_to_uri(file_path)
            if _state.workspace_state.get(pkg_uri) is not None:
                continue
            if _state.workspace_index.get_analysis(pkg_uri) is not None:
                continue
            scan_result = _state.background_scanner.rescan_file(file_path)
            if scan_result:
                _state.workspace_index.update(
                    pkg_uri,
                    scan_result.analysis,
                    EntrySource.PACKAGE,
                )


# Dialect / file-type detection


def _is_bigip_conf(uri: str) -> bool:
    return is_bigip_conf_name(uri)


def _is_irules_source(uri: str) -> bool:
    lang_id = _state.workspace_state.get_language_id(uri).lower()
    if lang_id in ("irules", "irul", "irule"):
        return True
    basename = uri.rsplit("/", 1)[-1].lower() if "/" in uri else uri.lower()
    return basename.endswith(".irul") or basename.endswith(".irule")


def _is_apl_source(uri: str) -> bool:
    lang_id = _state.workspace_state.get_language_id(uri).lower()
    if lang_id in ("tcl-apl", "apl-lang", "apl"):
        return True
    basename = uri.rsplit("/", 1)[-1].lower() if "/" in uri else uri.lower()
    return basename.endswith(".apl") or basename == "presentation"


# BIG-IP / APL specialised publishers


def _publish_bigip_diagnostics(
    uri: str,
    source: str,
    version: int | None = None,
) -> None:
    from dialects.f5.bigip.diagnostics import (
        get_bigip_diagnostics,
        get_bigip_lint_diagnostics,
    )
    from dialects.f5.bigip.parser import parse_bigip_conf

    try:
        config = parse_bigip_conf(source)
        _state.background_scanner.parse_bigip_source(uri, source)
    except Exception:
        log.debug("bigip: failed to parse %s", uri, exc_info=True)
        _require_server().text_document_publish_diagnostics(
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[], version=version)
        )
        return

    cfg = _state.config_for_uri(uri)
    if cfg.diagnostics_enabled:
        # Stanza-shape / value-format diagnostics (BIGIP6xxx codes,
        # always anchored on parser ranges).
        diagnostics = [
            _to_lsp_diagnostic(d)
            for d in get_bigip_diagnostics(config, disabled_codes=cfg.disabled_diagnostics)
        ]
        # Cross-file lint diagnostics (orphaned monitors, missing
        # iRule references, partition mismatches, deprecated commands,
        # …).  Driven by the same lint rule registry the ``f5 lint``
        # CLI uses, so adding a rule lights up the editor.  Only
        # findings whose target lives in *this* file get a range; the
        # rest surface when their owning file is opened.
        scanner_configs = (
            _state.background_scanner.bigip_configs
            if hasattr(_state, "background_scanner") and _state.background_scanner
            else None
        )
        # ``_get_doc_source`` looks each URI up via the LSP workspace
        # if open, falling back to file I/O otherwise.  Skip URIs where
        # the lookup returns empty (file gone, race with delete) so we
        # don't feed empty strings to the lint walker.
        scanner_sources: dict[str, str] | None = None
        if scanner_configs:
            scanner_sources = {}
            for u in scanner_configs:
                doc_source = _state._get_doc_source(u)
                if doc_source:
                    scanner_sources[u] = doc_source
        try:
            diagnostics = list(diagnostics) + [
                _to_lsp_diagnostic(d)
                for d in get_bigip_lint_diagnostics(
                    uri=uri,
                    source=source,
                    config=config,
                    workspace_sources=scanner_sources,
                    workspace_configs=scanner_configs,
                    disabled_codes=cfg.disabled_diagnostics,
                )
            ]
        except Exception:  # noqa: BLE001
            # Lint pipeline is best-effort — never let a buggy rule
            # take down the per-keystroke diagnostics path.
            log.debug("bigip lint: failed for %s", uri, exc_info=True)
    else:
        diagnostics = []

    _require_server().text_document_publish_diagnostics(
        types.PublishDiagnosticsParams(
            uri=uri,
            diagnostics=diagnostics,
            version=version,
        )
    )


def _uri_to_dir(uri: str) -> str | None:
    if uri.startswith("file://"):
        path = uri[7:]
    else:
        path = uri
    import os

    return os.path.dirname(path) if path else None


def _publish_apl_diagnostics(
    uri: str,
    source: str,
    version: int | None = None,
) -> None:
    from analyser.semantic_model import Severity
    from dialects.f5.bigip.iapp_diagnostics import validate_iapp_presentation
    from server._lsp_conv import to_lsp_range

    base_dir = _uri_to_dir(uri)
    model = _state.background_scanner.parse_apl_source(uri, source, base_dir)
    if model is None:
        _require_server().text_document_publish_diagnostics(
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[], version=version)
        )
        return

    cfg = _state.config_for_uri(uri)
    if not cfg.diagnostics_enabled:
        _require_server().text_document_publish_diagnostics(
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[], version=version)
        )
        return

    impl_var_refs = _find_sibling_impl_vars(uri, base_dir)

    severity_map = {
        Severity.ERROR: types.DiagnosticSeverity.Error,
        Severity.WARNING: types.DiagnosticSeverity.Warning,
        Severity.INFO: types.DiagnosticSeverity.Information,
        Severity.HINT: types.DiagnosticSeverity.Hint,
    }
    raw = validate_iapp_presentation(model, impl_var_refs)
    results: list[types.Diagnostic] = []
    for d in raw:
        if cfg.disabled_diagnostics and d.code in cfg.disabled_diagnostics:
            continue
        results.append(
            types.Diagnostic(
                range=to_lsp_range(d.range),
                message=d.message,
                severity=severity_map.get(d.severity, types.DiagnosticSeverity.Warning),
                source="tcl-lsp",
                code=d.code or None,
            )
        )
    _require_server().text_document_publish_diagnostics(
        types.PublishDiagnosticsParams(uri=uri, diagnostics=results, version=version)
    )


def _find_sibling_impl_vars(uri: str, base_dir: str | None) -> list | None:
    import os

    from dialects.f5.bigip.iapp_vars import extract_iapp_var_refs

    impl_uri = _state.background_scanner.find_sibling_impl_source(uri)
    if impl_uri is not None:
        try:
            impl_doc = _require_server().workspace.get_text_document(impl_uri)
            if impl_doc is not None:
                return extract_iapp_var_refs(impl_doc.source)
        except Exception:
            pass

    if not base_dir:
        return None

    candidates: list[str] = []
    impl_path = os.path.join(base_dir, "implementation")
    if os.path.isfile(impl_path):
        candidates.append(impl_path)
    try:
        for fname in os.listdir(base_dir):
            ext = os.path.splitext(fname)[1].lower()
            if ext in (".iapp", ".iappimpl", ".impl"):
                candidates.append(os.path.join(base_dir, fname))
    except OSError:
        pass

    for path in candidates:
        try:
            with open(path, encoding="utf-8", errors="replace") as f:
                return extract_iapp_var_refs(f.read())
        except OSError:
            continue
    return None

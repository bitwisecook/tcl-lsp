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

import lsp.state as _state

from .features.diagnostics import get_basic_diagnostics, get_deep_diagnostics, get_diagnostics
from .workspace.scanner import path_to_uri, uri_to_path
from .workspace.workspace_index import EntrySource

if TYPE_CHECKING:
    from pygls.lsp.server import LanguageServer

log = logging.getLogger(__name__)

# Server reference

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    global _server
    _server = server_instance


# Pull diagnostics cache

_pull_diag_cache: dict[str, list[types.Diagnostic]] = {}
_pull_diag_result_ids: dict[str, str] = {}
_pull_diag_counter = itertools.count(1)


def _next_pull_diag_result_id() -> str:
    return f"tcl-lsp-diag-{next(_pull_diag_counter)}"


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
    from core.analysis.semantic_model import WorkspaceDiagnosticContext

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
        from core.analysis.source_resolver import resolve_source_target

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
        from core.compiler.irules_flow import extract_rule_init_vars

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
    from core.analysis.checks._style import non_ascii_mode_scope
    from core.common.dialect import dialect_scope

    cfg = _state.config_for_uri(uri)
    dialect, extras = _state.resolve_dialect_for_uri(uri, source)
    with (
        dialect_scope(dialect=dialect, extra_commands=extras),
        non_ascii_mode_scope(cfg.non_ascii_mode),
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
    from core.analysis.checks._style import non_ascii_mode_scope
    from core.common.dialect import dialect_scope

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
    ):
        await _publish_diagnostics_inner(uri, source, version, force_reanalyse=force_reanalyse)


async def _publish_diagnostics_inner(
    uri: str,
    source: str,
    version: int | None = None,
    *,
    force_reanalyse: bool = False,
) -> None:
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
        is_fresh = state.analysis is None or force_reanalyse
        if is_fresh:
            from core.analysis.checks._style import _non_ascii_mode_var
            from core.commands.registry.runtime import _dialect_var, _extra_commands_var
            from lsp.workspace.document_state import _analyse_document_fresh

            try:
                loop = asyncio.get_running_loop()
                pool = _state._get_process_pool()
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
                        ),
                    ),
                    timeout=15.0,
                )
                state.apply_subprocess_result(result, version)
                subprocess_result = result
            except BrokenProcessPool:
                log.warning("Process pool broken, falling back to thread")
                _state._process_pool = None
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

    if state.version != version:
        log.info(
            "[timing] _publish_diagnostics abandoned (stale: have v%s, want v%s)",
            state.version,
            version,
        )
        return

    partial_mode = state.has_partial_commands

    if did_analyse and state.analysis is not None:
        try:
            _server.workspace_semantic_tokens_refresh(None)  # type: ignore[union-attr]
        except Exception:
            pass
        try:
            _server.workspace_folding_range_refresh(None)  # type: ignore[union-attr]
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

    from core.common.dialect import active_dialect
    from lsp.features.diagnostics import _run_deep_diagnostics

    _dialect = active_dialect()
    _pool = _state._get_process_pool()
    _generic_var_patterns = list(cfg.generic_variable_patterns)

    async def _deep_coro() -> list[types.Diagnostic]:
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
                ),
            )
        except BrokenProcessPool:
            log.warning("Process pool broken in deep diagnostics, falling back to thread")
            _state._process_pool = None
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
            )
        if _state_ref.version == _scheduled_version:
            _state_ref.store_deep_diagnostics(result)
        return result

    _state.diagnostic_scheduler.schedule_async(
        uri,
        version,
        basic_diags,
        _deep_coro,
        _publish_diags_to_client,
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
    from core.analysis.semantic_model import AnalysisResult

    if not isinstance(analysis, AnalysisResult):
        return

    # Per-folder PackageResolver selection (issue #407): documents in
    # folders with their own ``tclLsp.libraryPaths`` use a dedicated
    # resolver configured with those paths; everything else falls back to
    # the workspace-level resolver.
    resolver = _state.package_resolver_for_uri(uri)

    if analysis.auto_path_entries and uri is not None:
        import os

        from core.analysis.auto_path_eval import evaluate_auto_path_expr

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
    from .workspace.scanner import _BIGIP_CONF_NAMES

    basename = uri.rsplit("/", 1)[-1].lower() if "/" in uri else uri.lower()
    return basename in _BIGIP_CONF_NAMES


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
    from core.bigip.diagnostics import (
        get_bigip_diagnostics,
        get_bigip_lint_diagnostics,
    )
    from core.bigip.parser import parse_bigip_conf

    try:
        config = parse_bigip_conf(source)
        _state.background_scanner.parse_bigip_source(uri, source)
    except Exception:
        log.debug("bigip: failed to parse %s", uri, exc_info=True)
        _server.text_document_publish_diagnostics(  # type: ignore[union-attr]
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[], version=version)
        )
        return

    cfg = _state.config_for_uri(uri)
    if cfg.diagnostics_enabled:
        # Stanza-shape / value-format diagnostics (BIGIP6xxx codes,
        # always anchored on parser ranges).
        diagnostics = get_bigip_diagnostics(config, disabled_codes=cfg.disabled_diagnostics)
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
            diagnostics = list(diagnostics) + get_bigip_lint_diagnostics(
                uri=uri,
                source=source,
                config=config,
                workspace_sources=scanner_sources,
                workspace_configs=scanner_configs,
                disabled_codes=cfg.disabled_diagnostics,
            )
        except Exception:  # noqa: BLE001
            # Lint pipeline is best-effort — never let a buggy rule
            # take down the per-keystroke diagnostics path.
            log.debug("bigip lint: failed for %s", uri, exc_info=True)
    else:
        diagnostics = []

    _server.text_document_publish_diagnostics(  # type: ignore[union-attr]
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
    from core.analysis.semantic_model import Severity
    from core.bigip.iapp_diagnostics import validate_iapp_presentation
    from core.common.lsp import to_lsp_range

    base_dir = _uri_to_dir(uri)
    model = _state.background_scanner.parse_apl_source(uri, source, base_dir)
    if model is None:
        _server.text_document_publish_diagnostics(  # type: ignore[union-attr]
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[], version=version)
        )
        return

    cfg = _state.config_for_uri(uri)
    if not cfg.diagnostics_enabled:
        _server.text_document_publish_diagnostics(  # type: ignore[union-attr]
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
    _server.text_document_publish_diagnostics(  # type: ignore[union-attr]
        types.PublishDiagnosticsParams(uri=uri, diagnostics=results, version=version)
    )


def _find_sibling_impl_vars(uri: str, base_dir: str | None) -> list | None:
    import os

    from core.bigip.iapp_vars import extract_iapp_var_refs

    impl_uri = _state.background_scanner.find_sibling_impl_source(uri)
    if impl_uri is not None:
        try:
            impl_doc = _server.workspace.get_text_document(impl_uri)  # type: ignore[union-attr]
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

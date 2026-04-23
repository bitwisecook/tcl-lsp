"""Document lifecycle and file-system event handlers."""

from __future__ import annotations

import asyncio
import logging
import time
from typing import TYPE_CHECKING

from lsprotocol import types
from pygls.lsp.server import LanguageServer

from core.commands.registry.runtime import configure_signatures, is_irules_dialect

from .workspace.scanner import uri_to_path
from .workspace.workspace_index import EntrySource
from .features.workspace_file_ops import compute_batch_rename_edits

import lsp.state as _state
import lsp.diagnostics_pipeline as _dp

if TYPE_CHECKING:
    pass

log = logging.getLogger(__name__)

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    global _server
    _server = server_instance


# ---------------------------------------------------------------------------
# Document lifecycle handlers
# ---------------------------------------------------------------------------


async def did_open(params: types.DidOpenTextDocumentParams) -> None:
    t_open = time.perf_counter()
    uri = params.text_document.uri
    lang_id = params.text_document.language_id or ""
    n_lines = params.text_document.text.count("\n") + 1
    log.info("Opened %s (language_id=%r, lines=%d)", uri, lang_id, n_lines)
    _state.workspace_state.open(
        uri,
        params.text_document.text,
        params.text_document.version,
        language_id=lang_id,
        analyse=False,
    )

    if not is_irules_dialect() and _dp._is_irules_source(uri):
        log.info("Auto-switching to f5-irules dialect (language_id=%r)", lang_id)
        configure_signatures(dialect="f5-irules")
        _server.window_show_message(  # type: ignore[union-attr]
            types.ShowMessageParams(
                type=types.MessageType.Info,
                message="Switched to iRules dialect for F5 iRules support.",
            )
        )
    elif not _state.feature_config.dialect_explicitly_set:
        from core.common.dialect import detect_dialect_from_source

        source_dialect = detect_dialect_from_source(params.text_document.text)
        if source_dialect:
            changed = configure_signatures(dialect=source_dialect)
            if changed:
                log.info(
                    "Auto-switched to %s dialect (detected from source)",
                    source_dialect,
                )
        else:
            from lsp.workspace_init import _upgrade_dialect_from_workspace

            _upgrade_dialect_from_workspace()

    if _dp._is_bigip_conf(uri):
        _dp._publish_bigip_diagnostics(
            uri,
            params.text_document.text,
            params.text_document.version,
        )
        return
    if _dp._is_apl_source(uri):
        _dp._publish_apl_diagnostics(
            uri,
            params.text_document.text,
            params.text_document.version,
        )
        return
    asyncio.create_task(
        _dp._publish_diagnostics(
            uri,
            params.text_document.text,
            params.text_document.version,
        )
    )
    log.info("[timing] did_open total %.0fms (uri=%s)", (time.perf_counter() - t_open) * 1000, uri)


async def did_change(params: types.DidChangeTextDocumentParams) -> None:
    doc = _server.workspace.get_text_document(params.text_document.uri)  # type: ignore[union-attr]
    if _dp._is_bigip_conf(params.text_document.uri):
        _dp._publish_bigip_diagnostics(
            params.text_document.uri,
            doc.source,
            params.text_document.version,
        )
        return
    if _dp._is_apl_source(params.text_document.uri):
        _dp._publish_apl_diagnostics(
            params.text_document.uri,
            doc.source,
            params.text_document.version,
        )
        return
    await _dp._publish_diagnostics(
        params.text_document.uri,
        doc.source,
        params.text_document.version,
    )


def did_close(params: types.DidCloseTextDocumentParams) -> None:
    uri = params.text_document.uri
    log.info("Closed %s", uri)
    _state.diagnostic_scheduler.cancel(uri)
    if _dp._is_bigip_conf(uri):
        _state.background_scanner.remove_bigip_config(uri)
        _server.text_document_dp._publish_diagnostics(  # type: ignore[union-attr]
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[])
        )
        return
    if _dp._is_apl_source(uri):
        _state.background_scanner.remove_apl_model(uri)
        _server.text_document_dp._publish_diagnostics(  # type: ignore[union-attr]
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[])
        )
        return
    with _state._semantic_token_results_lock:
        _state._semantic_token_results.pop(uri, None)
    _state.workspace_state.close(uri)
    bg_analysis = _state.background_scanner.get_cached(uri)
    if bg_analysis is not None:
        _state.workspace_index.update(uri, bg_analysis, EntrySource.BACKGROUND)
    else:
        _state.workspace_index.remove(uri)
    _server.text_document_dp._publish_diagnostics(  # type: ignore[union-attr]
        types.PublishDiagnosticsParams(uri=uri, diagnostics=[])
    )


# ---------------------------------------------------------------------------
# Shutdown
# ---------------------------------------------------------------------------


def on_shutdown(params: None) -> None:
    """Cancel pending background tasks and shut down process pool."""
    _state.diagnostic_scheduler.cancel_all()
    if _state._process_pool is not None:
        _state._process_pool.shutdown(wait=False, cancel_futures=True)
        _state._process_pool = None


# ---------------------------------------------------------------------------
# File-system watch and rename hooks
# ---------------------------------------------------------------------------


def did_change_watched_files(
    params: types.DidChangeWatchedFilesParams,
) -> None:
    """React to file system changes for non-open files."""
    for change in params.changes:
        uri = change.uri
        if _state.workspace_state.get(uri) is not None:
            continue

        if change.type == types.FileChangeType.Deleted:
            _state.workspace_index.remove(uri)
            _state.background_scanner.remove_file(uri)
        elif change.type in (
            types.FileChangeType.Created,
            types.FileChangeType.Changed,
        ):
            file_path = uri_to_path(uri)
            if file_path:
                scan_result = _state.background_scanner.rescan_file(file_path)
                if scan_result:
                    _state.workspace_index.update(
                        uri,
                        scan_result.analysis,
                        EntrySource.BACKGROUND,
                    )
                    if scan_result.dialect_hint == "f5-irules":
                        _state.workspace_index.update_irules_globals(
                            uri,
                            scan_result.analysis.all_procs,
                        )
                        if scan_result.rule_init_exports:
                            _state.workspace_index.update_rule_init_vars(
                                uri,
                                scan_result.rule_init_exports,
                            )


_RENAME_FILE_OPERATION_OPTIONS = types.FileOperationRegistrationOptions(
    filters=[
        types.FileOperationFilter(
            pattern=types.FileOperationPattern(
                glob="**/*.{tcl,tm,itcl,irule,irul}",
            ),
        ),
    ],
)


def on_will_rename_files(
    params: types.RenameFilesParams,
) -> types.WorkspaceEdit | None:
    """Rewrite ``source`` lines in dependents so the workspace still loads."""
    if not _state.feature_config.workspace_file_ops_enabled:
        return None
    roots: list[str] = []
    ws = _server.workspace  # type: ignore[union-attr]
    if ws.root_path:
        roots.append(ws.root_path)
    return compute_batch_rename_edits(
        list(params.files),
        _state.workspace_index,
        workspace_roots=roots,
    )


def on_did_rename_files(params: types.RenameFilesParams) -> None:
    """Reindex renamed files after the client applies the rename on disk."""
    if not _state.feature_config.workspace_file_ops_enabled:
        return
    for f in params.files:
        old_uri = f.old_uri
        new_uri = f.new_uri
        _state.workspace_index.remove(old_uri)
        _state.background_scanner.remove_file(old_uri)
        new_path = uri_to_path(new_uri)
        if not new_path:
            continue
        scan_result = _state.background_scanner.rescan_file(new_path)
        if scan_result:
            _state.workspace_index.update(
                new_uri,
                scan_result.analysis,
                EntrySource.BACKGROUND,
            )


# ---------------------------------------------------------------------------
# Registration
# ---------------------------------------------------------------------------


def register(server_instance: LanguageServer) -> None:
    """Register all lifecycle handlers with the server."""
    configure(server_instance)
    server_instance.feature(types.TEXT_DOCUMENT_DID_OPEN)(did_open)
    server_instance.feature(types.TEXT_DOCUMENT_DID_CHANGE)(did_change)
    server_instance.feature(types.TEXT_DOCUMENT_DID_CLOSE)(did_close)
    server_instance.feature(types.SHUTDOWN)(on_shutdown)
    server_instance.feature(types.WORKSPACE_DID_CHANGE_WATCHED_FILES)(did_change_watched_files)
    server_instance.feature(
        types.WORKSPACE_WILL_RENAME_FILES, _RENAME_FILE_OPERATION_OPTIONS
    )(on_will_rename_files)
    server_instance.feature(
        types.WORKSPACE_DID_RENAME_FILES, _RENAME_FILE_OPERATION_OPTIONS
    )(on_did_rename_files)

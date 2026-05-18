"""Document lifecycle and file-system event handlers."""

from __future__ import annotations

import asyncio
import logging
import time

from lsprotocol import types
from pygls.lsp.server import LanguageServer

import lsp.diagnostics_pipeline as _dp
import lsp.state as _state
from core.commands.registry.runtime import configure_signatures
from core.common.dialect import active_dialect

from .features.workspace_file_ops import compute_batch_rename_edits
from .workspace.scanner import uri_to_path
from .workspace.workspace_index import EntrySource

log = logging.getLogger(__name__)

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    global _server
    _server = server_instance


# Document lifecycle handlers


async def did_open(params: types.DidOpenTextDocumentParams) -> None:
    t_open = time.perf_counter()
    uri = params.text_document.uri
    lang_id = params.text_document.language_id or ""
    n_lines = params.text_document.text.count("\n") + 1
    had_open_documents = bool(_state.workspace_state.items())
    log.info("Opened %s (language_id=%r, lines=%d)", uri, lang_id, n_lines)
    state = _state.workspace_state.open(
        uri,
        params.text_document.text,
        params.text_document.version,
        language_id=lang_id,
        analyse=False,
    )

    folder_cfg = _state.config_for_uri(uri)
    # Per-folder auto-switch (issue #407): the auto-switch only matters when
    # *this folder's resolved dialect* is not already iRules.  Gating on
    # ``is_irules_dialect()`` (the process default) produced spurious
    # "Auto-switching to f5-irules" notifications for folders that were
    # already explicitly configured for iRules.
    folder_effective_dialect = folder_cfg.dialect or active_dialect()
    if folder_effective_dialect != "f5-irules" and _dp._is_irules_source(uri):
        # Skip entirely when the folder's dialect was explicitly set —
        # the user has chosen a non-iRules dialect for this folder, so
        # auto-switching would override that choice silently and a
        # "Switched to iRules dialect" notification would be misleading
        # because we don't actually mutate the folder config in that case.
        if folder_cfg.dialect_explicitly_set:
            log.info(
                "Skipping iRules auto-switch for %s: folder dialect is explicitly %s",
                uri,
                folder_cfg.dialect,
            )
        else:
            log.info("Auto-switching to f5-irules dialect (language_id=%r)", lang_id)
            folder_cfg.dialect = "f5-irules"
            if folder_cfg is _state.feature_config:
                configure_signatures(dialect="f5-irules")
            _server.window_show_message(  # type: ignore[union-attr]
                types.ShowMessageParams(
                    type=types.MessageType.Info,
                    message="Switched to iRules dialect for F5 iRules support.",
                )
            )
    elif not folder_cfg.dialect_explicitly_set:
        # Use the document's cached dialect hint (state.dialect_hint is
        # populated by workspace_state.open() via
        # WorkspaceDocumentState.refresh_dialect_hint).  Per-folder
        # dialect always reflects the document's hint — subsequent
        # requests in this folder go through ``resolve_dialect_for_uri``
        # which picks it up.  The workspace-wide default
        # (``configure_signatures``) only auto-switches on the *first*
        # opened document so later files with conflicting hints don't
        # ping-pong the process-default ContextVar.
        source_dialect = state.dialect_hint
        if source_dialect:
            folder_cfg.dialect = source_dialect
            if not had_open_documents:
                changed = configure_signatures(dialect=source_dialect)
                if changed:
                    log.info(
                        "Auto-switched workspace dialect to %s for first opened document",
                        source_dialect,
                    )
            else:
                log.info(
                    "Set per-document dialect %s for %s; workspace dialect unchanged",
                    source_dialect,
                    uri,
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
    # Drop cached hover entries so a reopen-with-version-reset (clients
    # commonly reset to ``1``) cannot serve stale entries from the
    # previous session keyed on a matching ``(uri, version, line, char)``.
    from lsp.server import _invalidate_hover_cache

    _invalidate_hover_cache(uri)
    if _dp._is_bigip_conf(uri):
        _state.background_scanner.remove_bigip_config(uri)
        _server.text_document_publish_diagnostics(  # type: ignore[union-attr]
            types.PublishDiagnosticsParams(uri=uri, diagnostics=[])
        )
        return
    if _dp._is_apl_source(uri):
        _state.background_scanner.remove_apl_model(uri)
        _server.text_document_publish_diagnostics(  # type: ignore[union-attr]
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
    _server.text_document_publish_diagnostics(  # type: ignore[union-attr]
        types.PublishDiagnosticsParams(uri=uri, diagnostics=[])
    )


# Shutdown


def on_shutdown(params: None) -> None:
    """Cancel pending background tasks and shut down process pool."""
    _state.diagnostic_scheduler.cancel_all()
    if _state._process_pool is not None:
        _state._process_pool.shutdown(wait=False, cancel_futures=True)
        _state._process_pool = None


# File-system watch and rename hooks


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


def on_did_change_workspace_folders(params: types.DidChangeWorkspaceFoldersParams) -> None:
    """Track folder add/remove and re-pull per-folder configuration."""
    from core.common.user_config import get_all_settings, load_project_config
    from lsp.settings import _pull_and_apply_configuration, _schedule_apply_merged

    from .workspace.scanner import uri_to_path as _uri_to_path

    event = params.event
    removed_uris: list[str] = []
    for folder in getattr(event, "removed", None) or []:
        folder_uri = getattr(folder, "uri", "")
        if folder_uri:
            _state.drop_folder_configs(folder_uri)
            removed_uris.append(folder_uri)

    for folder in getattr(event, "added", None) or []:
        folder_uri = getattr(folder, "uri", "")
        folder_path = _uri_to_path(folder_uri) if folder_uri else None
        if not folder_uri:
            continue
        # Initialise per-folder configs and load .tcl-lsp.ini if present.
        _state.get_or_init_folder_feature_config(folder_uri)
        _state.get_or_init_folder_formatter_config(folder_uri)
        if folder_path:
            project_settings = get_all_settings(load_project_config(folder_path), kind="project")
            _state.project_config_settings_per_folder[folder_uri] = project_settings

    # If the fallback project layer was sourced from a removed folder
    # (or any folder was removed at all), clear it so files outside every
    # workspace folder don't see stale project settings.  ``on_initialized``
    # arbitrarily picks the first folder's ``.tcl-lsp.ini`` as the fallback;
    # rather than re-pick on every change, drop it back to "none" so the
    # global user-config layer is the only fallback contributor.
    if removed_uris:
        _state.project_config_settings = {}

    # Schedule a merged-settings apply now so per-folder analysis state
    # reflects the new folder set even if ``_pull_and_apply_configuration``
    # below fails (e.g. client without ``workspace/configuration`` support).
    _schedule_apply_merged()
    _pull_and_apply_configuration()


# Registration


def register(server_instance: LanguageServer) -> None:
    """Register all lifecycle handlers with the server."""
    configure(server_instance)
    server_instance.feature(types.TEXT_DOCUMENT_DID_OPEN)(did_open)
    server_instance.feature(types.TEXT_DOCUMENT_DID_CHANGE)(did_change)
    server_instance.feature(types.TEXT_DOCUMENT_DID_CLOSE)(did_close)
    server_instance.feature(types.SHUTDOWN)(on_shutdown)
    server_instance.feature(types.WORKSPACE_DID_CHANGE_WATCHED_FILES)(did_change_watched_files)
    server_instance.feature(types.WORKSPACE_DID_CHANGE_WORKSPACE_FOLDERS)(
        on_did_change_workspace_folders
    )
    server_instance.feature(types.WORKSPACE_WILL_RENAME_FILES, _RENAME_FILE_OPERATION_OPTIONS)(
        on_will_rename_files
    )
    server_instance.feature(types.WORKSPACE_DID_RENAME_FILES, _RENAME_FILE_OPERATION_OPTIONS)(
        on_did_rename_files
    )

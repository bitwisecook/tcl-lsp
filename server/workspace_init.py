"""Workspace initialisation: background scanning, dialect auto-upgrade, progress."""

from __future__ import annotations

import asyncio
import logging
import threading
import time
from collections.abc import Callable
from typing import TYPE_CHECKING

from lsprotocol import types
from pygls.lsp.server import LanguageServer

import server.state as _state
from compiler.registry.runtime import configure_signatures
from shared.user_config import (
    get_all_settings,
    load_project_config,
    load_user_config,
)
from tooling.formatter import FormatterConfig

from .workspace.scanner import path_to_uri
from .workspace.scanner import uri_to_path as _uri_to_path
from .workspace.workspace_index import EntrySource

if TYPE_CHECKING:
    pass

log = logging.getLogger(__name__)

try:
    from ._build_info import FULL_VERSION as _version
except ImportError:
    _version = "dev"

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    global _server
    _server = server_instance


def _require_server() -> LanguageServer:
    if _server is None:
        raise RuntimeError("lsp.workspace_init: server not configured")
    return _server


# Import warming


def _warm_imports() -> None:
    """Eagerly import heavy modules to eliminate cold-start latency."""
    t0 = time.perf_counter()
    try:
        import analyser  # noqa: F811, F401
        import compiler.compilation_unit  # noqa: F811, F401
        import server.features._semantic_tokens  # noqa: F811, F401
        import server.features.diagnostics  # noqa: F811, F401

        elapsed_ms = (time.perf_counter() - t0) * 1000
        log.info("[timing] import warming %.0fms", elapsed_ms)
    except Exception:
        log.debug("Import warming failed (non-fatal)", exc_info=True)


# Background scan

_scan_lock = threading.Lock()

_TCL_VERSION_ORDER = {"tcl8.4": 0, "tcl8.5": 1, "tcl8.6": 2, "tcl9.0": 3}


def _upgrade_dialect_from_workspace() -> None:
    """Upgrade the global dialect if any workspace file requires a higher Tcl version."""
    from shared.dialect import active_dialect

    current = active_dialect()
    current_rank = _TCL_VERSION_ORDER.get(current, 2)
    best_dialect = current
    best_rank = current_rank

    for uri in _state.workspace_index.all_uris():
        analysis = _state.workspace_index.get_analysis(uri)
        if analysis is None:
            continue
        for pr in analysis.package_requires:
            if pr.name != "Tcl":
                continue
            ver = pr.version or ""
            ver_clean = ver.rstrip("-")
            dialect_key = f"tcl{ver_clean}" if not ver_clean.startswith("tcl") else ver_clean
            rank = _TCL_VERSION_ORDER.get(dialect_key)
            if rank is not None and rank > best_rank:
                best_rank = rank
                best_dialect = dialect_key

    if best_dialect != current:
        changed = configure_signatures(dialect=best_dialect)
        if changed:
            log.info(
                "Auto-upgraded dialect to %s (detected from workspace Tcl version)",
                best_dialect,
            )


def _run_background_scan(
    progress_cb: Callable[[int, int, str], None] | None = None,
) -> None:
    """Execute background scan in a daemon thread."""
    if not _scan_lock.acquire(blocking=False):
        log.debug("Background scan already running — skipping duplicate request")
        return
    t0 = time.perf_counter()
    try:
        open_uris = frozenset(uri for uri, _ in _state.workspace_state.items())
        results = _state.background_scanner.scan_all(skip_uris=open_uris, progress_cb=progress_cb)
        for uri, scan_result in results.items():
            if _state.workspace_state.get(uri) is not None:
                continue
            _state.workspace_index.update(uri, scan_result.analysis, EntrySource.BACKGROUND)
        for uri, procs in _state.background_scanner.irules_procs.items():
            _state.workspace_index.update_irules_globals(uri, procs)
        for uri, exports in _state.background_scanner.irules_rule_init_vars.items():
            _state.workspace_index.update_rule_init_vars(uri, exports)
        auto_loaded = 0
        for _proc_name, source_file in _state.background_scanner.auto_index_entries.items():
            source_uri = path_to_uri(source_file)
            if _state.workspace_index.get_analysis(source_uri) is not None:
                continue
            scan_result = _state.background_scanner.rescan_file(source_file)
            if scan_result:
                _state.workspace_index.update(
                    source_uri,
                    scan_result.analysis,
                    EntrySource.AUTO_INDEX,
                )
                auto_loaded += 1
        if not _state.feature_config.dialect_explicitly_set:
            _upgrade_dialect_from_workspace()
        elapsed_ms = (time.perf_counter() - t0) * 1000
        log.info(
            "[timing] background scan %.0fms (%d files, %d auto-index)",
            elapsed_ms,
            len(results),
            auto_loaded,
        )
    except Exception:
        log.error("Background scan failed", exc_info=True)
    finally:
        _scan_lock.release()


# Strong reference to prevent GC of the coordinator task between scans.
_scan_progress_task: "asyncio.Task[None] | None" = None


def _start_scan_with_progress() -> None:
    """Run the background scan with $/progress notifications."""
    import uuid

    global _scan_progress_task

    loop = getattr(_server, "loop", None)
    if loop is None or not loop.is_running():
        threading.Thread(target=_run_background_scan, daemon=True).start()
        return

    token = f"tcl-lsp-scan-{uuid.uuid4()}"
    progress_active = False

    def _progress_cb(idx: int, total: int, path: str) -> None:
        if not progress_active:
            return
        percentage = int((idx / total) * 100) if total else 0

        def _send() -> None:
            try:
                _require_server().work_done_progress.report(
                    token,
                    types.WorkDoneProgressReport(
                        message=f"{idx}/{total}",
                        percentage=percentage,
                    ),
                )
            except Exception:
                pass

        try:
            loop.call_soon_threadsafe(_send)
        except Exception:
            pass

    async def _coordinator() -> None:
        nonlocal progress_active
        try:
            await asyncio.wait_for(
                _require_server().work_done_progress.create_async(token),
                timeout=5.0,
            )
        except (Exception, asyncio.TimeoutError):
            log.debug(
                "work_done_progress/create failed or timed out; running scan silently",
                exc_info=True,
            )
            threading.Thread(target=_run_background_scan, daemon=True).start()
            return

        try:
            _require_server().work_done_progress.begin(
                token,
                types.WorkDoneProgressBegin(
                    title="Scanning workspace",
                    cancellable=False,
                    percentage=0,
                ),
            )
            progress_active = True
        except Exception:
            threading.Thread(target=_run_background_scan, daemon=True).start()
            return

        done = threading.Event()

        def _worker() -> None:
            try:
                _run_background_scan(progress_cb=_progress_cb)
            finally:
                done.set()

        threading.Thread(target=_worker, daemon=True).start()

        while not done.is_set():
            await asyncio.sleep(0.1)

        progress_active = False
        try:
            _require_server().work_done_progress.end(
                token,
                types.WorkDoneProgressEnd(message="done"),
            )
        except Exception:
            pass

    _scan_progress_task = loop.create_task(_coordinator())


# on_initialized handler


def on_initialized(params: types.InitializedParams) -> None:
    """After client initialization, scan workspace for Tcl files."""
    from server.settings import (
        _apply_merged_settings_now,
        _normalise_formatter_settings,
        _pull_and_apply_configuration,
    )
    from shared.dialect import active_dialect

    _state.global_config_settings = get_all_settings(load_user_config(), kind="global")
    # Formatter config is rebuilt fresh from global here rather than via the
    # layered apply because the layered path only *updates* a dict — a baseline
    # FormatterConfig instance needs to exist first.
    formatting = _state.global_config_settings.get("formatting")
    if isinstance(formatting, dict) and formatting:
        _state.formatter_config = FormatterConfig.from_dict(
            _normalise_formatter_settings(formatting)
        )

    # Discover project-level config files: one per workspace folder for
    # multi-root workspaces, plus the workspace fallback for files outside
    # any folder.
    ws = _require_server().workspace
    # pygls 2.x exposes folders as ``Workspace.folders`` (dict keyed by URI);
    # older releases used ``workspace_folders`` (list).  Support both.
    _folders_attr = getattr(ws, "folders", None)
    if _folders_attr is None:
        _folders_attr = getattr(ws, "workspace_folders", None) or []
    if isinstance(_folders_attr, dict):
        folders = list(_folders_attr.values())
    else:
        folders = list(_folders_attr)
    folder_paths: list[tuple[str, str]] = []
    for folder in folders:
        folder_uri = getattr(folder, "uri", "")
        folder_path = _uri_to_path(folder_uri)
        if folder_uri and folder_path:
            folder_paths.append((folder_uri, folder_path))

    if folder_paths:
        for folder_uri, folder_path in folder_paths:
            project_settings = get_all_settings(load_project_config(folder_path), kind="project")
            _state.project_config_settings_per_folder[folder_uri] = project_settings
            # Initialise per-folder configs so later pulls and lookups have
            # somewhere to write into.
            _state.get_or_init_folder_feature_config(folder_uri)
            _state.get_or_init_folder_formatter_config(folder_uri)
        # Fallback (no scope) uses the first folder's project config so
        # files outside any folder still get a sensible default.
        _state.project_config_settings = (
            _state.project_config_settings_per_folder[folder_paths[0][0]] if folder_paths else {}
        )
    else:
        project_settings: dict = {}
        if ws.root_path:
            project_settings = get_all_settings(load_project_config(ws.root_path), kind="project")
        _state.project_config_settings = project_settings

    # Apply the current (global + project) layers so server state is
    # populated before the first analysis pass.  Editor settings are
    # pulled per workspace folder near the end of this handler — the
    # callback re-runs ``_apply_merged_settings_now`` with the populated
    # editor layer.
    _apply_merged_settings_now()

    process_start = getattr(_server, "_process_start_time", None)
    if process_start is not None:
        startup_ms = (time.monotonic() - process_start) * 1000
        log.info("[timing] server ready: %.0fms from process start", startup_ms)

    log.info(
        "Server initialized (version=%s, dialect=%s)",
        _version,
        active_dialect(),
    )

    caps = _require_server().client_capabilities
    st = getattr(getattr(caps, "text_document", None), "semantic_tokens", None)
    if st is None:
        log.info("Client did not advertise semantic token support")
        _require_server().window_show_message(
            types.ShowMessageParams(
                type=types.MessageType.Info,
                message=(
                    "Tip: enable semantic tokens for richer Tcl highlighting. "
                    'In Zed, add \'"semantic_tokens": "full"\' to your '
                    "language settings."
                ),
            )
        )

    roots: list[str] = []
    ws = _require_server().workspace
    if ws.root_path:
        roots.append(ws.root_path)

    library_paths: list[str] = []
    if ws.root_path:
        import os

        _tclpkg_dir = None
        _search = ws.root_path
        for _ in range(10):
            if os.path.isfile(os.path.join(_search, "tclpkg.tcl")):
                _tclpkg_dir = _search
                break
            _parent = os.path.dirname(_search)
            if _parent == _search:
                break
            _search = _parent

        if _tclpkg_dir is not None:
            log.info("Detected tclpkg project at: %s", _tclpkg_dir)
            project_lib = os.path.join(_tclpkg_dir, "lib")
            if os.path.isdir(project_lib):
                library_paths.append(project_lib)
        venv_dir = os.environ.get("TCL_VENV")
        if not venv_dir:
            local_venv = os.path.join(ws.root_path, ".venv", "tclvenv.cfg")
            if os.path.isfile(local_venv):
                venv_dir = os.path.join(ws.root_path, ".venv")
        if venv_dir:
            venv_lib = os.path.join(venv_dir, "lib")
            if os.path.isdir(venv_lib):
                library_paths.append(venv_lib)
                log.info("Auto-detected tclpkg venv lib: %s", venv_lib)

    _state.background_scanner.configure(
        workspace_roots=roots,
        library_paths=library_paths if library_paths else None,
    )
    resolver_paths = list(roots) + list(library_paths)
    if resolver_paths:
        _state.package_resolver.configure(search_paths=resolver_paths)
        _state._loaded_packages.clear()

    progress_supported = bool(
        _state.feature_config.progress_enabled
        and getattr(getattr(caps, "window", None), "work_done_progress", False)
    )
    if progress_supported:
        _start_scan_with_progress()
    else:
        threading.Thread(target=_run_background_scan, daemon=True).start()

    threading.Thread(target=_warm_imports, daemon=True).start()

    _pull_and_apply_configuration()


# Registration


def register(server_instance: LanguageServer) -> None:
    """Register the initialized handler with the server."""
    configure(server_instance)
    server_instance.feature(types.INITIALIZED)(on_initialized)

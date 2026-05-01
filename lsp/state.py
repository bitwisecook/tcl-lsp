"""Shared singleton state and utility helpers for the Tcl LSP server."""

from __future__ import annotations

import itertools
import logging
import multiprocessing
import threading
from concurrent.futures import ProcessPoolExecutor
from typing import TYPE_CHECKING

import core.common.codes_all  # noqa: F401  # must precede feature_config import so profile categories are computed with full registry
from core.formatting import FormatterConfig
from core.packages import PackageResolver

from .async_diagnostics import DiagnosticScheduler
from .feature_config import FeatureConfig
from .workspace.document_state import WorkspaceState
from .workspace.scanner import BackgroundScanner
from .workspace.workspace_index import WorkspaceIndex

if TYPE_CHECKING:
    from pygls.lsp.server import LanguageServer

log = logging.getLogger(__name__)

# Singleton state

workspace_state = WorkspaceState()
workspace_index = WorkspaceIndex()
background_scanner = BackgroundScanner()
package_resolver = PackageResolver()
formatter_config = FormatterConfig()
feature_config = FeatureConfig()
diagnostic_scheduler = DiagnosticScheduler()

# Per-folder configuration overrides (issue #230).
#
# Each workspace folder URI may have its own ``FeatureConfig`` and
# ``FormatterConfig`` that override the workspace/user-level fallback
# (``feature_config`` / ``formatter_config`` above).  Documents outside
# every known folder, and workspace-wide queries with no URI, use the
# fallback.
_per_folder_feature_configs: dict[str, FeatureConfig] = {}
_per_folder_formatter_configs: dict[str, FormatterConfig] = {}


def _longest_folder_prefix(doc_uri: str, folders: list[str]) -> str | None:
    """Return the longest workspace-folder URI that prefixes ``doc_uri``."""
    best: str | None = None
    for folder in folders:
        normalised = folder.rstrip("/")
        if doc_uri == normalised or doc_uri.startswith(normalised + "/"):
            if best is None or len(normalised) > len(best.rstrip("/")):
                best = folder
    return best


def workspace_folder_uris() -> list[str]:
    """Workspace folder URIs that have per-folder config overrides."""
    return list(_per_folder_feature_configs.keys())


def config_for_uri(doc_uri: str | None) -> FeatureConfig:
    """Resolve the effective ``FeatureConfig`` for a document URI.

    Picks the longest matching workspace-folder URI; falls back to the
    workspace/user-level ``feature_config`` when the document is outside
    every known folder or no URI was supplied.
    """
    if doc_uri and _per_folder_feature_configs:
        match = _longest_folder_prefix(doc_uri, list(_per_folder_feature_configs.keys()))
        if match is not None:
            return _per_folder_feature_configs[match]
    return feature_config


def formatter_config_for_uri(doc_uri: str | None) -> FormatterConfig:
    """Resolve the effective ``FormatterConfig`` for a document URI."""
    if doc_uri and _per_folder_formatter_configs:
        match = _longest_folder_prefix(doc_uri, list(_per_folder_formatter_configs.keys()))
        if match is not None:
            return _per_folder_formatter_configs[match]
    return formatter_config


def get_or_init_folder_feature_config(folder_uri: str) -> FeatureConfig:
    """Initialise the FeatureConfig for ``folder_uri`` if missing, then return it.

    A new folder inherits a deep copy of the fallback so it starts aligned
    with workspace/user defaults.
    """
    from copy import deepcopy

    if folder_uri not in _per_folder_feature_configs:
        _per_folder_feature_configs[folder_uri] = deepcopy(feature_config)
    return _per_folder_feature_configs[folder_uri]


def get_or_init_folder_formatter_config(folder_uri: str) -> FormatterConfig:
    """Initialise the FormatterConfig for ``folder_uri`` if missing, then return it."""
    from copy import deepcopy

    if folder_uri not in _per_folder_formatter_configs:
        _per_folder_formatter_configs[folder_uri] = deepcopy(formatter_config)
    return _per_folder_formatter_configs[folder_uri]


def set_folder_formatter_config(folder_uri: str, config: FormatterConfig) -> None:
    """Replace the FormatterConfig for ``folder_uri`` (or the fallback when "")."""
    global formatter_config
    if folder_uri == "":
        formatter_config = config
    else:
        _per_folder_formatter_configs[folder_uri] = config


def drop_folder_configs(folder_uri: str) -> None:
    """Drop per-folder configs and editor/project layers for a closed folder."""
    _per_folder_feature_configs.pop(folder_uri, None)
    _per_folder_formatter_configs.pop(folder_uri, None)
    editor_config_settings_per_folder.pop(folder_uri, None)
    project_config_settings_per_folder.pop(folder_uri, None)


# Configuration layers, merged on each apply in the order:
#   global_config_settings  ← ``~/.config/tcl-lsp/config.ini`` (lowest priority,
#                              user-home — applies everywhere)
#   editor_config_settings  ← ``workspace/configuration`` pull payload
#                              (per workspace folder; fallback under "")
#   project_config_settings ← ``<folder>/.tcl-lsp.ini`` (highest priority,
#                              per workspace folder; fallback under "")
#
# See ``docs/kcs/kcs-howto-suppress-diagnostics.md`` for the full precedence
# chain including inline ``# <noqa>`` and file-level ``# tcl-lsp: disable=``
# directives that override all server-level configuration.
global_config_settings: dict = {}
editor_config_settings: dict = {}
project_config_settings: dict = {}
editor_config_settings_per_folder: dict[str, dict] = {}
project_config_settings_per_folder: dict[str, dict] = {}

_process_pool: ProcessPoolExecutor | None = None


def _get_process_pool() -> ProcessPoolExecutor:
    """Lazy singleton ProcessPoolExecutor for CPU-intensive analysis.

    Uses "forkserver" on platforms that support it to avoid deadlocks
    when forking a multi-threaded process (asyncio + pygls threads).
    The default "fork" start method can deadlock when a thread holds
    a lock at fork time.
    """
    global _process_pool
    if _process_pool is None:
        try:
            ctx = multiprocessing.get_context("forkserver")
        except ValueError:
            ctx = None  # Windows — use default
        _process_pool = ProcessPoolExecutor(max_workers=2, mp_context=ctx)
    return _process_pool


_loaded_packages: set[str] = set()
_SAFE_FIX_CODES = frozenset(
    {
        "W100",
        "W105",
        "W108",
        "W110",
        "W201",
        "W304",
        "IRULE2001",
    }
)

# Semantic token delta cache (shared between server.py handlers and lifecycle)

# Per-URI storage for semantic tokens delta support.
# Maps URI → (result_id, flat token data).
_semantic_token_results: dict[str, tuple[str, list[int]]] = {}
_semantic_token_results_lock = threading.Lock()
# Thread-safe counter — pygls may dispatch handlers concurrently.
_semantic_token_result_counter = itertools.count(1)

# Server reference injection (for _get_doc_source fallback)

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    """Inject the LanguageServer instance so _get_doc_source can fall back to it."""
    global _server
    _server = server_instance


# Utility helpers


def _get_doc_source(uri: str) -> str:
    """Get document source text, handling virtual documents without backing files.

    Prefers the in-memory ``DocumentState`` source (always available for
    documents opened via ``textDocument/didOpen``).  Falls back to the pygls
    ``TextDocument`` which may read from disk.  Returns an empty string for
    virtual or untitled documents that have no backing file.
    """
    state = workspace_state.get(uri)
    if state is not None:
        return state.source
    if _server is None:
        return ""
    try:
        doc = _server.workspace.get_text_document(uri)
        return doc.source
    except OSError:
        return ""
    except Exception:
        log.debug("Unexpected error reading document source for %s", uri, exc_info=True)
        return ""

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

# Per-folder PackageResolver instances (issue #407).  When a workspace
# folder defines its own ``tclLsp.libraryPaths`` the lazily-built resolver
# in this map is configured with those paths plus the workspace_roots; the
# fallback ``package_resolver`` above remains in use for documents outside
# every known folder.
_per_folder_package_resolvers: dict[str, PackageResolver] = {}


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


def dialect_scope_for_uri(doc_uri: str | None, source: str | None = None):
    """Return a ``dialect_scope`` context manager for ``doc_uri``.

    Convenience wrapper that resolves the folder-scoped dialect (and
    extra_commands) via :func:`resolve_dialect_for_uri` and opens a
    ``dialect_scope`` so request handlers can simply do::

        with _state.dialect_scope_for_uri(uri, source):
            ...
    """
    from core.common.dialect import dialect_scope

    dialect, extras = resolve_dialect_for_uri(doc_uri, source)
    return dialect_scope(dialect=dialect, extra_commands=extras)


def _uri_from_lsp_params(params) -> str | None:
    """Best-effort URI extraction from an LSP request ``params`` object.

    Most ``textDocument/*`` requests carry the URI at
    ``params.text_document.uri``.  A few request shapes nest it differently
    (call-hierarchy/type-hierarchy resolve items, code-lens resolve, etc.).
    This helper accepts any of the common shapes and returns ``None`` if no
    URI is reachable so the caller can fall back to the process default.
    """
    td = getattr(params, "text_document", None)
    if td is not None:
        uri = getattr(td, "uri", None)
        if uri:
            return uri
    item = getattr(params, "item", None)
    if item is not None:
        uri = getattr(item, "uri", None)
        if uri:
            return uri
    direct_uri = getattr(params, "uri", None)
    if isinstance(direct_uri, str) and direct_uri:
        return direct_uri
    data = getattr(params, "data", None)
    if isinstance(data, dict):
        d_uri = data.get("uri")
        if isinstance(d_uri, str) and d_uri:
            return d_uri
    return None


def _maybe_doc_source(uri: str) -> str | None:
    """Return the cached source for ``uri`` if the workspace knows it.

    Used by ``scoped_to_doc`` so the in-source dialect-detection step in
    :func:`resolve_dialect_for_uri` (shebang, ``# tcl-dialect:`` directive,
    ``package require Tcl X.Y``, conf-wrapped iRules) applies to every LSP
    request — matching the diagnostics pipeline's behaviour.  Never raises
    or falls through to a pygls workspace lookup: handlers tolerate the
    source being absent (most don't need it for the resolver anyway since
    the resolved dialect is also cached on ``FeatureConfig.dialect`` by
    ``did_open``).
    """
    state = workspace_state.get(uri)
    return state.source if state is not None and state.source is not None else None


def scoped_to_doc(fn):
    """Decorator: open ``dialect_scope`` for the document URI in ``params``.

    Applies to LSP request handlers so that per-folder dialect resolution
    (issue #407) is honoured by every step of the handler — including any
    lazy re-lex, signature lookup, or compiler re-entry that consults
    ``active_dialect()`` / ``SIGNATURES``.  Passes the document's cached
    source through to :func:`resolve_dialect_for_uri` so priority-1
    in-source hints (``# tcl-dialect:`` etc.) are honoured consistently
    with the diagnostics pipeline.  Handlers whose ``params`` carry no
    URI (workspace-wide queries) run under the process default.
    """
    import functools
    import inspect

    if inspect.iscoroutinefunction(fn):

        @functools.wraps(fn)
        async def async_wrapper(params, *args, **kwargs):
            uri = _uri_from_lsp_params(params)
            if uri is None:
                return await fn(params, *args, **kwargs)
            with dialect_scope_for_uri(uri, _maybe_doc_source(uri)):
                return await fn(params, *args, **kwargs)

        return async_wrapper

    @functools.wraps(fn)
    def sync_wrapper(params, *args, **kwargs):
        uri = _uri_from_lsp_params(params)
        if uri is None:
            return fn(params, *args, **kwargs)
        with dialect_scope_for_uri(uri, _maybe_doc_source(uri)):
            return fn(params, *args, **kwargs)

    return sync_wrapper


def resolve_dialect_for_uri(
    doc_uri: str | None,
    source: str | None = None,
) -> tuple[str | None, tuple[str, ...] | None]:
    """Resolve ``(dialect, extra_commands)`` for the given document URI.

    Consults, in priority order:

    1. ``detect_dialect_from_source(source)`` — in-source hints (shebang,
       ``# tcl-dialect:`` directive, ``package require Tcl X.Y``, conf-wrapped
       iRules).
    2. The folder-scoped ``FeatureConfig`` (longest workspace-folder prefix
       match) — its ``dialect`` / ``extra_commands`` fields when set.
    3. The workspace-fallback ``FeatureConfig`` — same fields.

    Each component may independently be ``None`` (meaning "inherit the
    process default that ``configure_signatures`` set at server startup /
    on workspace config change").  Callers wrap their work in
    :func:`core.common.dialect.dialect_scope` with the result; ``None``
    values leave the corresponding ContextVar untouched.
    """
    from core.common.dialect import detect_dialect_from_source

    dialect: str | None = None
    extras: tuple[str, ...] | None = None

    if source is not None:
        detected = detect_dialect_from_source(source)
        if detected is not None:
            dialect = detected

    cfg = config_for_uri(doc_uri)
    if dialect is None and cfg.dialect is not None:
        dialect = cfg.dialect
    if extras is None and cfg.extra_commands is not None:
        extras = cfg.extra_commands

    # Fall back to the workspace-level FeatureConfig if the folder-specific
    # one inherited a None.
    if cfg is not feature_config:
        if dialect is None and feature_config.dialect is not None:
            dialect = feature_config.dialect
        if extras is None and feature_config.extra_commands is not None:
            extras = feature_config.extra_commands

    return dialect, extras


def package_resolver_for_uri(doc_uri: str | None) -> PackageResolver:
    """Resolve the effective :class:`PackageResolver` for a document URI.

    Picks the longest matching workspace-folder URI; falls back to the
    workspace/user-level :data:`package_resolver` when the document is
    outside every known folder, no URI was supplied, or the folder's
    ``FeatureConfig.library_paths`` was cleared (so a stale resolver
    with old search paths doesn't keep getting selected after the user
    unsets ``tclLsp.libraryPaths``).
    """
    if doc_uri and _per_folder_package_resolvers:
        match = _longest_folder_prefix(doc_uri, list(_per_folder_package_resolvers.keys()))
        if match is not None:
            folder_cfg = _per_folder_feature_configs.get(match)
            if folder_cfg is not None and folder_cfg.library_paths:
                return _per_folder_package_resolvers[match]
    return package_resolver


def get_or_init_folder_package_resolver(folder_uri: str) -> PackageResolver:
    """Initialise the PackageResolver for ``folder_uri`` if missing.

    New folder resolvers inherit a fresh empty state; the caller is
    responsible for calling ``configure(...)`` with the folder's
    libraryPaths + workspace_roots before the first ``resolve`` call.
    """
    if folder_uri not in _per_folder_package_resolvers:
        _per_folder_package_resolvers[folder_uri] = PackageResolver()
    return _per_folder_package_resolvers[folder_uri]


def all_package_resolvers() -> list[PackageResolver]:
    """Return every active PackageResolver (workspace fallback + per-folder).

    Used by workspace-wide queries that need to enumerate packages
    available anywhere in the workspace (e.g. ``tclLsp.listPackages``).
    """
    return [package_resolver, *_per_folder_package_resolvers.values()]


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
    # Also prune the per-folder PackageResolver and any cached "loaded"
    # entries keyed to it, so workspace-wide queries (``listPackages`` /
    # package suggestions) stop returning packages from a folder that's
    # no longer open.
    dropped_resolver = _per_folder_package_resolvers.pop(folder_uri, None)
    if dropped_resolver is not None:
        dropped_key = id(dropped_resolver)
        _loaded_packages.difference_update(
            entry for entry in list(_loaded_packages) if entry[0] == dropped_key
        )


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


# Tracks which (resolver, package_name) pairs have already had their source
# files loaded into the workspace index.  Keyed by ``id(resolver)`` so each
# per-folder ``PackageResolver`` (issue #407) can independently track its
# own loaded packages — otherwise folder A loading ``Foo 1.0`` would
# silently block folder B from loading its own ``Foo 2.0``.
_loaded_packages: set[tuple[int, str]] = set()
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

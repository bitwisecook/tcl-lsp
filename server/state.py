# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Shared singleton state and utility helpers for the Tcl LSP server."""

from __future__ import annotations

import itertools
import logging
import multiprocessing
import os
import threading
from concurrent.futures import ProcessPoolExecutor
from typing import TYPE_CHECKING

import server._codes_init  # noqa: F401  # must precede feature_config import so profile categories are computed with full registry
from analyser.packages import PackageResolver
from compiler.registry.dialect import (
    detect_dialect_directive,
    detect_dialect_from_source,
    dialect_scope,
)
from tooling.formatter import FormatterConfig

from .async_diagnostics import DiagnosticScheduler
from .feature_config import FeatureConfig
from .workspace.document_state import WorkspaceState
from .workspace.scanner import BackgroundScanner, is_bigip_conf_name
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


def document_buffer_for(uri: str | None, source: str):
    """The document's live rope-backed buffer for *source* — the single
    per-document buffer — or a freshly built one when the document isn't tracked
    or its current text differs from *source*.

    Lets request handlers read the one spliced buffer (``update_source_quick``
    keeps it edit-spliced in O(log n)) instead of rebuilding the O(n) position
    index per request.  Position-identical by construction: the live buffer is
    used only when ``buffer.source == source`` exactly, else a fresh build.
    """
    from shared.document_buffer import DocumentBuffer

    if uri is not None:
        state = workspace_state.get(uri)
        if state is not None:
            buf = state.buffer
            if buf.source == source:
                return buf
    return DocumentBuffer.from_source(source)


# Per-folder configuration overrides (issue #230).
#
# Each workspace folder URI may have its own ``FeatureConfig`` and
# ``FormatterConfig`` that override the workspace/user-level fallback
# (``feature_config`` / ``formatter_config`` above).  Documents outside
# every known folder, and workspace-wide queries with no URI, use the
# fallback.
_per_folder_feature_configs: dict[str, FeatureConfig] = {}
_per_folder_formatter_configs: dict[str, FormatterConfig] = {}

# Workspace-level command stubs discovered from external ``.tcl.stubs``
# files at workspace init.  Applied to every analysed document so the
# analyser understands EDA / dialect commands the same way it does for
# inline ``# tcl-lsp: stub`` blocks.  Empty when no stub files exist.
workspace_stub_commands: list = []

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

    1. An explicit ``# tcl-dialect:`` directive in the source — the user's
       override always wins.
    2. BIG-IP configuration file detection (by basename → ``"f5-bigip"``).
       This beats the rest of source autodetection so a ``bigip.conf`` that
       embeds ``ltm rule`` stanzas is not misclassified as ``f5-irules`` by
       the conf-wrapped-iRules heuristic.
    3. The remaining in-source hints (shebang, ``package require Tcl X.Y``,
       conf-wrapped iRules) via ``detect_dialect_from_source(source)``.
    4. The folder-scoped ``FeatureConfig`` (longest workspace-folder prefix
       match) — its ``dialect`` / ``extra_commands`` fields when set.
    5. The workspace-fallback ``FeatureConfig`` — same fields.

    Steps 1-3 mirror ``infer_document_dialect`` so the two resolvers never
    disagree about a document's dialect.

    Each component may independently be ``None`` (meaning "inherit the
    process default that ``configure_signatures`` set at server startup /
    on workspace config change").  Callers wrap their work in
    :func:`compiler.registry.dialect.dialect_scope` with the result; ``None``
    values leave the corresponding ContextVar untouched.
    """
    dialect: str | None = None
    extras: tuple[str, ...] | None = None

    # 1. Explicit ``# tcl-dialect:`` directive — the user's override wins.
    if source is not None:
        dialect = detect_dialect_directive(source)

    # 2. BIG-IP configuration files (bigip.conf, bigip_base.conf, …) are not
    #    Tcl source — they are key-value config stanzas that often embed
    #    ``ltm rule`` stanzas.  Resolve their dialect to ``"f5-bigip"`` ahead
    #    of source autodetection so the conf-wrapped-iRules heuristic in
    #    ``detect_dialect_from_source`` cannot misclassify them as
    #    ``f5-irules`` (which would defeat the f5-bigip analysis skip).
    if dialect is None and doc_uri is not None and is_bigip_conf_name(doc_uri):
        dialect = "f5-bigip"

    # 3. Remaining in-source hints (shebang, package require, conf-wrapped).
    if dialect is None and source is not None:
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
    with workspace/user defaults — except for ``dialect_explicitly_set``,
    which is per-scope: an explicit dialect choice at the workspace level
    is *not* an explicit choice for this folder, so the folder starts
    with the flag cleared.  Otherwise ``did_open``'s iRules / iApps
    auto-switch would be silently disabled for every folder created
    after a workspace-level ``setDialect`` (e.g. opening a ``.irul``
    file in a workspace whose top-level was previously pinned to a
    Tcl version).
    """
    from copy import deepcopy

    if folder_uri not in _per_folder_feature_configs:
        cfg = deepcopy(feature_config)
        cfg.dialect_explicitly_set = False
        _per_folder_feature_configs[folder_uri] = cfg
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
_deep_pool: ProcessPoolExecutor | None = None
_small_pool: ProcessPoolExecutor | None = None

# Cold full builds scale with the machine (a multi-root workspace / a
# git-checkout that reopens several tabs submits several multi-second builds at
# once; a 2-worker cap made a trivial file wait behind them).  Deep diagnostics
# get a *separate* small pool so a cold-build storm can't starve every open
# document's deep pass — the two no longer share a queue (head-of-line blocking).
#
# Small fresh builds get a THIRD pool (the "small-file lane").  CPU-bound
# analysis fans out badly in threads (the GIL makes N concurrent in-thread
# builds slower than serial — measured 4-thread 20.8s vs serial 15.3s vs
# 4-process 7.7s), so a fresh build runs in a subprocess for true parallelism.
# Small builds use their own pool, separate from the cold pool, so a trivial
# file never queues behind a workspace of multi-second cold builds.
_COLD_POOL_WORKERS = max(2, min(4, (os.cpu_count() or 4) - 1))
_DEEP_POOL_WORKERS = 2
_SMALL_POOL_WORKERS = max(2, min(4, (os.cpu_count() or 4) - 1))


def _forkserver_context():
    """A "forkserver" multiprocessing context (avoids fork deadlocks when a
    thread holds a lock at fork time); ``None`` on platforms without it."""
    try:
        return multiprocessing.get_context("forkserver")
    except ValueError:
        return None  # Windows — use default


def _get_process_pool() -> ProcessPoolExecutor:
    """Lazy pool for cold full builds (``_analyse_document_fresh``).

    Kept separate from :func:`_get_deep_pool` so a burst of multi-second cold
    builds cannot block deep diagnostics workspace-wide.
    """
    global _process_pool
    if _process_pool is None:
        _process_pool = ProcessPoolExecutor(
            max_workers=_COLD_POOL_WORKERS, mp_context=_forkserver_context()
        )
    return _process_pool


def _reset_process_pool() -> None:
    """Tear down the cold-build pool so the next build gets a fresh one.

    Used to poison-and-recreate after a wedged build (ceiling exceeded) or a
    ``BrokenProcessPool``.  ``shutdown(wait=False, cancel_futures=True)`` releases
    queued-but-unstarted futures immediately and stops new work landing on a
    wedged worker; an already-running orphaned build can't be cancelled but is no
    longer referenced and is reclaimed when its process exits.
    """
    global _process_pool
    pool, _process_pool = _process_pool, None
    if pool is not None:
        try:
            pool.shutdown(wait=False, cancel_futures=True)
        except Exception:
            pass  # best-effort pool shutdown; ignore if workers already stopped


def _get_deep_pool() -> ProcessPoolExecutor:
    """Lazy pool for deep diagnostics (``_run_deep_diagnostics``).

    Separate from the cold-build pool (:func:`_get_process_pool`) so deep passes
    stay responsive while large files cold-build.
    """
    global _deep_pool
    if _deep_pool is None:
        _deep_pool = ProcessPoolExecutor(
            max_workers=_DEEP_POOL_WORKERS, mp_context=_forkserver_context()
        )
    return _deep_pool


def _get_small_pool() -> ProcessPoolExecutor:
    """Lazy pool for *small* fresh full builds (``_analyse_document_fresh``).

    The small-file lane: separate from the cold-build pool
    (:func:`_get_process_pool`) so a trivial file's fresh build runs in parallel
    and never queues behind a workspace of multi-second cold builds.  A fresh
    build runs in a subprocess (not a thread) because CPU-bound analysis is
    GIL-bound — in-thread fan-out is slower than serial.
    """
    global _small_pool
    if _small_pool is None:
        _small_pool = ProcessPoolExecutor(
            max_workers=_SMALL_POOL_WORKERS, mp_context=_forkserver_context()
        )
    return _small_pool


def _reset_small_pool() -> None:
    """Tear down the small-build pool so the next build gets a fresh one.

    Mirrors :func:`_reset_process_pool` for the small-file lane: poison-and-
    recreate after a wedged build (ceiling exceeded) or a ``BrokenProcessPool``.
    """
    global _small_pool
    pool, _small_pool = _small_pool, None
    if pool is not None:
        try:
            pool.shutdown(wait=False, cancel_futures=True)
        except Exception:
            pass  # best-effort small-pool shutdown; ignore if workers already stopped


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

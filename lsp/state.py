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


def config_for_uri(doc_uri: str | None) -> FeatureConfig:  # noqa: ARG001
    """Resolve the effective ``FeatureConfig`` for a document URI.

    PR A introduces this seam as a shim that returns the global
    ``feature_config``.  Per-folder resolution lands in the follow-up
    refactor (issue #230 PR B), where this helper picks the longest
    workspace-folder-prefix match from a folder→config map.
    """
    return feature_config

# Configuration layers, merged on each apply in the order:
#   global_config_settings  ← ``~/.config/tcl-lsp/config.ini`` (lowest priority)
#   editor_config_settings  ← ``workspace/didChangeConfiguration`` payload
#   project_config_settings ← ``<workspace>/.tcl-lsp.ini`` (highest priority)
#
# See ``docs/kcs/kcs-howto-suppress-diagnostics.md`` for the full precedence
# chain including inline ``# <noqa>`` and file-level ``# tcl-lsp: disable=``
# directives that override all server-level configuration.
global_config_settings: dict = {}
editor_config_settings: dict = {}
project_config_settings: dict = {}

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

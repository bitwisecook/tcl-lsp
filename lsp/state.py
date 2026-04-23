"""Shared singleton state and utility helpers for the Tcl LSP server."""

from __future__ import annotations

import itertools
import multiprocessing
import re
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

# ---------------------------------------------------------------------------
# Singleton state
# ---------------------------------------------------------------------------

workspace_state = WorkspaceState()
workspace_index = WorkspaceIndex()
background_scanner = BackgroundScanner()
package_resolver = PackageResolver()
formatter_config = FormatterConfig()
feature_config = FeatureConfig()
diagnostic_scheduler = DiagnosticScheduler()

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

# ---------------------------------------------------------------------------
# Semantic token delta cache (shared between server.py handlers and lifecycle)
# ---------------------------------------------------------------------------

# Per-URI storage for semantic tokens delta support.
# Maps URI → (result_id, flat token data).
_semantic_token_results: dict[str, tuple[str, list[int]]] = {}
_semantic_token_results_lock = threading.Lock()
# Thread-safe counter — pygls may dispatch handlers concurrently.
_semantic_token_result_counter = itertools.count(1)

# ---------------------------------------------------------------------------
# Server reference injection (for _get_doc_source fallback)
# ---------------------------------------------------------------------------

_server: LanguageServer | None = None


def configure(server_instance: LanguageServer) -> None:
    """Inject the LanguageServer instance so _get_doc_source can fall back to it."""
    global _server
    _server = server_instance


# ---------------------------------------------------------------------------
# Utility helpers
# ---------------------------------------------------------------------------


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
    doc = _server.workspace.get_text_document(uri)
    try:
        return doc.source
    except (FileNotFoundError, OSError):
        return ""


def _camel_to_snake(name: str) -> str:
    """Convert lowerCamelCase/PascalCase names to snake_case."""
    first = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", first).lower()


def _normalise_formatter_settings(raw: dict) -> dict:
    """Map client formatter settings to FormatterConfig field names."""
    normalised: dict[str, object] = {}
    for key, value in raw.items():
        if not isinstance(key, str):
            continue
        field = _camel_to_snake(key)
        if field == "line_ending" and isinstance(value, str):
            mapping = {
                "lf": "\n",
                "crlf": "\r\n",
                "cr": "\r",
            }
            value = mapping.get(value.lower(), value)
        normalised[field] = value
    return normalised


# Keys that appear directly inside the ``tclLsp`` configuration namespace.
# Used both for flat-key routing and for detecting unwrapped payloads from
# clients that strip the ``tclLsp`` prefix (e.g. JetBrains, vscode-languageclient v10).
_KNOWN_TCL_LSP_SECTIONS = frozenset(
    {
        "formatting",
        "diagnostics",
        "optimiser",
        "shimmer",
        "features",
        "style",
        "xcDiagnostics",
        "runtimeValidation",
        "ai",
        "packageManager",
    }
)
_KNOWN_TCL_LSP_TOPLEVEL = frozenset(
    {
        "dialect",
        "extraCommands",
        "libraryPaths",
    }
)


def _extract_tcl_lsp_settings(settings: dict) -> dict:
    """Extract extension/server settings from multiple client payload shapes.

    Handles three payload formats:
    1. Nested:   ``{"tclLsp": {"optimiser": {"O109": false}}}``
    2. Flat:     ``{"tclLsp.optimiser.O109": false}``
    3. Unwrapped (no ``tclLsp`` prefix — e.g. JetBrains pull-model response):
       ``{"optimiser": {"O109": false}, "dialect": "tcl8.6"}``
    """
    extracted: dict[str, object] = {}

    nested = settings.get("tclLsp")
    if isinstance(nested, dict):
        extracted.update(nested)

    for key, value in settings.items():
        if not isinstance(key, str):
            continue
        if key.startswith("tclLsp."):
            subkey = key[len("tclLsp."):]
        else:
            continue

        # Route dotted subkeys into nested dicts for known sections.
        section_handled = False
        for section in _KNOWN_TCL_LSP_SECTIONS:
            prefix = section + "."
            if subkey.startswith(prefix):
                section_key = subkey[len(prefix):]
                current = extracted.get(section)
                if not isinstance(current, dict):
                    current = {}
                    extracted[section] = current
                current[section_key] = value
                section_handled = True
                break
        if not section_handled:
            extracted[subkey] = value

    # Fallback: detect unwrapped payloads from clients that already stripped
    # the ``tclLsp`` prefix (e.g. workspace/configuration pull responses or
    # JetBrains didChangeConfiguration notifications).
    if not extracted:
        if any(
            isinstance(k, str) and k in _KNOWN_TCL_LSP_SECTIONS | _KNOWN_TCL_LSP_TOPLEVEL
            for k in settings
        ):
            extracted.update(settings)

    return extracted

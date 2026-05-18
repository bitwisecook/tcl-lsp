"""Entry point: python -m lsp"""

import logging
import time

_PROCESS_START = time.monotonic()

_t0 = time.perf_counter()
from .server import server  # noqa: E402

_import_ms = (time.perf_counter() - _t0) * 1000
logging.getLogger(__name__).info("[timing] server module import: %.0fms", _import_ms)

# Expose the startup timestamp so on_initialized can log total startup time.
# Dynamic attribute on pygls' LanguageServer — read back by lsp.workspace_init.
server._process_start_time = _PROCESS_START  # type: ignore[attr-defined]  # ty: ignore[unresolved-attribute]

server.start_io()

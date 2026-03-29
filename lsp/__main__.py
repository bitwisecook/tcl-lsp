"""Entry point: python -m lsp

Prefers the native C++ server binary when available.  Falls back to the
pygls-based Python server for environments without a compiled native binary.
"""

import logging
import os
import shutil
import sys
import time

_PROCESS_START = time.monotonic()

log = logging.getLogger(__name__)


def _find_native_binary():
    """Locate the native tcl-lsp-server binary."""
    # Check common build output locations relative to the project root.
    project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    candidates = [
        os.path.join(project_root, "builddir", "native", "tcl-lsp-server"),
        os.path.join(project_root, "build", "native", "tcl-lsp-server"),
    ]
    for path in candidates:
        if os.path.isfile(path) and os.access(path, os.X_OK):
            return path

    # Check PATH.
    found = shutil.which("tcl-lsp-server")
    if found:
        return found

    return None


def _run_native(binary_path):
    """Exec the native server binary, replacing this process."""
    log.info("Launching native server: %s", binary_path)
    try:
        os.execv(binary_path, [binary_path])
    except OSError:
        # Fall through to pygls fallback.
        log.warning("Failed to exec native binary, falling back to pygls")
        return False
    return True  # unreachable after successful execv


def main():
    # Allow forcing a specific mode via environment variable.
    mode = os.environ.get("TCL_LSP_MODE", "auto")

    if mode == "python":
        _run_pygls()
        return

    if mode == "native":
        binary = _find_native_binary()
        if binary:
            _run_native(binary)
        else:
            print("tcl-lsp: native binary not found and TCL_LSP_MODE=native", file=sys.stderr)
            sys.exit(1)
        return

    # Auto mode: prefer native, fall back to pygls.
    binary = _find_native_binary()
    if binary:
        _run_native(binary)
        # If we get here, execv failed — fall through.

    _run_pygls()


def _run_pygls():
    """Start the pygls-based Python server (legacy fallback)."""
    _t0 = time.perf_counter()
    try:
        from .server import server  # noqa: E402
    except ImportError as exc:
        message = (
            "tcl-lsp: Python (pygls-based) language server backend is not available.\n"
            "Install the pygls dependency, for example:\n"
            "  pip install 'pygls>=2.0'"
        )
        log.error("%s\n(ImportError: %s)", message, exc)
        print(message, file=sys.stderr)
        sys.exit(1)

    _import_ms = (time.perf_counter() - _t0) * 1000
    log.info("[timing] server module import: %.0fms", _import_ms)

    server._process_start_time = _PROCESS_START  # type: ignore[attr-defined]
    server.start_io()


main()

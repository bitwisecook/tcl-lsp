"""Route the analyser/diagnostics test entry points through the native Rust
LSP server, for measuring Phase-2 (diagnostic accuracy/precision) parity.

Activated by ``TCL_LSP_DIAG_BACKEND=rust`` (plus ``TCL_LSP_SERVER_BIN`` /
``TCL_LSP_SERVER_KIND=rust`` so the e2e harness client spawns the native
binary).  When active, ``server.features.diagnostics.get_diagnostics`` and
``analyser.analyse(...).diagnostics`` return the diagnostics the native
server publishes for the source, rebuilt as real
``lsprotocol.types.Diagnostic`` objects — exposing ``.code``, ``.message``,
``.severity`` and ``.range`` (with ``.start``/``.end`` →
``.line``/``.character``), the attributes the ``test_fp_*`` /
``test_ground_truth_tn_fn`` battery reads — so the value matches the
``list[types.Diagnostic]`` contract its caller declares.

The server is started once (module singleton) and driven with a single,
reused document URI (full-document ``didChange`` per call, version-bumped) so
no cross-document workspace state accumulates between reproducers.
"""

from __future__ import annotations

import os
import threading
from pathlib import Path
from typing import Any

from lsprotocol import types

_REPO = Path(__file__).resolve().parent.parent
_LOCK = threading.Lock()
_CLIENT: Any = None
_VERSION = 0
_URI = "file:///rust_diag_bridge_probe.tcl"


def active() -> bool:
    return os.environ.get("TCL_LSP_DIAG_BACKEND") == "rust"


def _client():
    global _CLIENT
    if _CLIENT is not None:
        return _CLIENT
    import sys

    sys.path.insert(0, str(_REPO / "tests" / "lsp_e2e"))
    from harness import LspServerClient, native_server_bin, server_launch_argv  # type: ignore

    native = native_server_bin()
    if native is None or not native.exists():
        raise RuntimeError(
            "TCL_LSP_DIAG_BACKEND=rust but no native tcl-lsp-server binary found; "
            "set TCL_LSP_SERVER_BIN or run `make rust-server`."
        )
    client = LspServerClient(server_launch_argv(native), _REPO)
    client.start()
    client.initialize(root_uri=_REPO.as_uri())
    _CLIENT = client
    return client


def _to_diagnostic(d: dict) -> types.Diagnostic:
    rng = d.get("range") or {}
    start = rng.get("start") or {}
    end = rng.get("end") or {}
    severity = d.get("severity")
    return types.Diagnostic(
        range=types.Range(
            start=types.Position(line=start.get("line", 0), character=start.get("character", 0)),
            end=types.Position(line=end.get("line", 0), character=end.get("character", 0)),
        ),
        message=d.get("message") or "",
        severity=types.DiagnosticSeverity(severity) if severity is not None else None,
        code=d.get("code"),
    )


def rust_diagnostics(source: str, language_id: str = "tcl") -> list[types.Diagnostic]:
    global _VERSION
    with _LOCK:
        client = _client()
        _VERSION += 1
        version = _VERSION
        if version == 1:
            diags = client.open_ready(_URI, source, language_id=language_id, version=version)
        else:
            client.replace_document(_URI, version, source)
            diags = client.await_diagnostics(_URI, version=version, timeout=30.0)
            client.await_log("workspace_state.update", _URI, timeout=30.0)
        return [_to_diagnostic(d) for d in diags]

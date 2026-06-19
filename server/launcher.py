"""``tcl-lsp`` console-script launcher: native Rust server by default.

The out-of-the-box backend is the native Rust ``tcl-lsp-server`` binary.
``launch()`` resolves that binary and ``os.execv``s it (replacing this
process so stdio is handed straight through); when no binary can be found
it falls back to the in-process Python reference server so the command
always works.

Backend selection mirrors the rest of the project:

* ``TCL_LSP_SERVER_KIND=python`` forces the Python reference server.
* ``TCL_LSP_SERVER_KIND=rust`` (or ``native``) requires the native binary
  and errors out if it cannot be found.
* unset / anything else → native binary if available, else Python.

``TCL_LSP_SERVER_BIN`` overrides binary resolution; otherwise ``PATH`` and
the repo's ``target/{release,debug}/`` are probed (the latter covers a
plain ``make rust-server`` checkout).
"""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path


def _repo_root() -> Path:
    # server/launcher.py -> server/ -> repo root.
    return Path(__file__).resolve().parent.parent


def native_server_bin() -> Path | None:
    """Resolve the native ``tcl-lsp-server`` binary, or ``None``.

    Honours ``TCL_LSP_SERVER_BIN`` first, then ``PATH``, then a
    release/debug build under the repo's ``target/``.
    """
    explicit = os.environ.get("TCL_LSP_SERVER_BIN")
    if explicit:
        candidate = Path(explicit).expanduser()
        return candidate if candidate.exists() else None

    exe = "tcl-lsp-server.exe" if sys.platform == "win32" else "tcl-lsp-server"
    on_path = shutil.which(exe)
    if on_path:
        return Path(on_path)

    for profile in ("release", "debug"):
        candidate = _repo_root() / "target" / profile / exe
        if candidate.exists():
            return candidate
    return None


def launch() -> None:
    """Entry point for the ``tcl-lsp`` console script."""
    kind = os.environ.get("TCL_LSP_SERVER_KIND", "").strip().lower()

    if kind == "python":
        from .__main__ import main as python_main

        python_main()
        return

    explicit_rust = kind in {"rust", "native"}
    binary = native_server_bin()

    if binary is not None:
        argv = [str(binary), *sys.argv[1:]]
        os.execv(str(binary), argv)
        return  # unreachable after execv; kept for clarity

    if explicit_rust:
        sys.stderr.write(
            "tcl-lsp: TCL_LSP_SERVER_KIND requests the native server but no "
            "tcl-lsp-server binary was found. Build it with `make rust-server` "
            "(or `cargo build -p tcl-lsp-server`), or set TCL_LSP_SERVER_BIN.\n"
        )
        raise SystemExit(1)

    # Default backend is native, but fall back to the Python reference
    # server so the command still works without a built binary.
    from .__main__ import main as python_main

    python_main()


if __name__ == "__main__":
    launch()

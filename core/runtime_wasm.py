"""Locate (and auto-build if needed) the Zig runtime WASM artifact.

The compiled ``runtime/zig/zig-out/bin/tcl_runtime.wasm`` is NOT
checked into git — it's a build artefact rebuilt from source.  This
module is the single point of truth for finding it:

* :data:`DEFAULT_PATH` — the standard build-output location
  (overridable via the ``TCL_LSP_RUNTIME_WASM`` environment variable
  for sweeps that want a custom build, e.g. the leak-check variant).

* :func:`runtime_wasm_path` — return the path, building the runtime
  on first call if the file is missing.  Subsequent calls are a
  no-op stat check.

* :func:`build_runtime` — explicit ``zig build`` invocation.  Honours
  the ``TCL_LSP_RUNTIME_BUILD_OPTS`` env var so callers can pass
  ``-Dleak-check=true`` for the leakcheck pipeline.

Tests, scripts, and the LSP server all route through this so any
fresh checkout (no committed binary) picks up automatically.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

# Repo root = two levels up from this file (.../core/runtime_wasm.py).
_REPO_ROOT = Path(__file__).resolve().parent.parent

DEFAULT_PATH: Path = (
    _REPO_ROOT / "runtime" / "zig" / "zig-out" / "bin" / "tcl_runtime.wasm"
)


def runtime_wasm_path(*, build_if_missing: bool = True) -> Path:
    """Return the path to the runtime WASM, building it if absent.

    The ``TCL_LSP_RUNTIME_WASM`` environment variable overrides the
    default location; the override is returned verbatim and never
    auto-built (callers using a custom path are responsible for
    keeping it fresh — typically a leak-check or alternative-config
    build the harness juggles separately).
    """
    override = os.environ.get("TCL_LSP_RUNTIME_WASM")
    if override:
        return Path(override)
    if build_if_missing and not DEFAULT_PATH.exists():
        build_runtime()
    return DEFAULT_PATH


def build_runtime(*, build_opts: list[str] | None = None) -> Path:
    """Run ``zig build`` in ``runtime/zig`` and return the artefact.

    Extra build flags can be passed via ``build_opts`` or the
    ``TCL_LSP_RUNTIME_BUILD_OPTS`` env var (whitespace-separated).
    The latter wins when both are present (so a CI / leakcheck
    invocation can force flags from the outside).
    """
    zig_dir = _REPO_ROOT / "runtime" / "zig"
    cmd = ["zig", "build"]
    if build_opts:
        cmd.extend(build_opts)
    env_opts = os.environ.get("TCL_LSP_RUNTIME_BUILD_OPTS", "").split()
    if env_opts:
        cmd.extend(env_opts)
    subprocess.run(cmd, cwd=str(zig_dir), check=True)
    if not DEFAULT_PATH.exists():
        msg = (
            f"build_runtime: zig build completed but {DEFAULT_PATH} "
            "was not produced"
        )
        raise FileNotFoundError(msg)
    return DEFAULT_PATH

"""Post-codegen bundling: merge runtime + user code into one binary.

The Tcl→WASM compiler emits a standalone user-code module that imports
from the ``"tcl"`` namespace (see :mod:`._imports`).  At deploy time
those imports must be resolved against an actual runtime.  Today the
host (``fuzzing/wasm_backend.py``, ``tests/runtime/_host_imports.py``)
re-exports runtime functions under ``"tcl"`` at instantiation; the
bundler does the same job at compile time, producing a single ``.wasm``
that needs no host-side wiring.

Mechanics: Binaryen's ``wasm-merge`` reads each input as a module with
a *given* import-namespace name and resolves inter-module imports
whose ``(module, field)`` matches an export of an earlier input.  The
pipeline below feeds it:

1. The runtime ``.wasm`` (lean ``tcl_runtime.wasm`` or the variant
   with extensions baked in, e.g. ``tcl_runtime_with_tcltest.wasm`` —
   selected by :func:`extensions.runtime_path_for`).  Named ``tcl``.
2. The user-code ``.wasm``.  Its ``import "tcl" "*"`` calls collapse
   into intra-module calls.

After the merge, the only remaining imports are WASI plus the small
``env.*`` host-bridge surface (``call_compiled_proc``, ``host_spawn``)
that hosts must still provide.

Note: optional extensions are *not* merged as separate WASMs in the
current model — instead, a runtime variant carries the extension's
commands compiled in.  See ``docs/design/compiler/wasm-extensions.md``
for the rationale (multi-memory/data-placement issues with
separately-merged extension WASMs).
"""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path


class WasmMergeUnavailableError(RuntimeError):
    """Raised when ``wasm-merge`` cannot be located on ``PATH``."""


class WasmMergeFailedError(RuntimeError):
    """Raised when ``wasm-merge`` exits non-zero."""


def _wasm_merge_path() -> str:
    """Locate ``wasm-merge`` on ``PATH``.

    Raises :class:`WasmMergeUnavailableError` if unavailable so callers
    can fall back to shipping the runtime + user wasm separately.
    """
    path = shutil.which("wasm-merge")
    if path is None:
        msg = (
            "wasm-merge (Binaryen) not found on PATH.  Install Binaryen "
            "v123 or later — the SessionStart hook places it at "
            "/usr/local/bin/wasm-merge in remote sessions; locally, "
            "install it from github.com/WebAssembly/binaryen and put "
            "wasm-merge on PATH."
        )
        raise WasmMergeUnavailableError(msg)
    return path


def _wasm_opt_path() -> str:
    """Locate ``wasm-opt`` (used for DWARF stripping pre-merge)."""
    path = shutil.which("wasm-opt")
    if path is None:
        msg = (
            "wasm-opt (Binaryen) not found on PATH; it ships alongside "
            "wasm-merge so this should not happen on a normal install."
        )
        raise WasmMergeUnavailableError(msg)
    return path


def _strip_dwarf(in_path: Path, out_path: Path, wasm_opt: str) -> None:
    """Copy *in_path* to *out_path* with DWARF debug info stripped.

    Binaryen's ``wasm-merge`` (as of v123) trips on the DWARF emitted
    by Zig 0.16 debug builds — ``Fatal: TODO: DW_LNE_define_file``.
    The asyncify post-pass in ``runtime/zig/build.zig`` strips DWARF
    for exactly this reason; we follow suit at bundle time.  The
    names section is preserved (``-g``) for readable stack traces.
    """
    proc = subprocess.run(  # noqa: S603 — argv is fully controlled
        [
            wasm_opt,
            "--strip-dwarf",
            "--all-features",
            "-g",
            str(in_path),
            "-o",
            str(out_path),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        msg = (
            f"wasm-opt --strip-dwarf exited {proc.returncode}\n"
            f"input: {in_path}\nstderr:\n{proc.stderr}"
        )
        raise WasmMergeFailedError(msg)


def bundle_wasm(
    user_wasm_bytes: bytes,
    *,
    runtime_path: Path,
) -> bytes:
    """Merge runtime + user code into a single WASM binary.

    Args:
        user_wasm_bytes: The raw bytes produced by
            :meth:`WasmModule.to_bytes` for the compiled user program.
        runtime_path: Path to the runtime ``.wasm`` whose exports
            satisfy the user module's ``"tcl"``-namespaced imports.
            Should be the variant returned by
            :func:`extensions.runtime_path_for` so the bundle covers
            every requested ``package require`` extension.

    Returns:
        Bytes of a single merged ``.wasm``.

    Raises:
        WasmMergeUnavailableError: ``wasm-merge`` not on PATH.
        WasmMergeFailedError: the merge invocation exited non-zero.
        FileNotFoundError: the runtime path is missing.
    """
    if not runtime_path.is_file():
        msg = (
            f"Runtime artifact not found: {runtime_path}.  Build it via "
            f"`cd runtime/zig && zig build` before bundling."
        )
        raise FileNotFoundError(msg)

    merge_bin = _wasm_merge_path()
    opt_bin = _wasm_opt_path()

    with tempfile.TemporaryDirectory(prefix="tcl_bundle_") as tmp_str:
        tmp = Path(tmp_str)

        user_raw = tmp / "user.raw.wasm"
        user_raw.write_bytes(user_wasm_bytes)

        # Strip DWARF from each input — wasm-merge v123 can't ingest
        # the DWARF Zig 0.16 debug builds emit.  Names section stays.
        runtime_stripped = tmp / "runtime.wasm"
        _strip_dwarf(runtime_path, runtime_stripped, opt_bin)
        user_stripped = tmp / "user.wasm"
        _strip_dwarf(user_raw, user_stripped, opt_bin)

        out_path = tmp / "bundled.wasm"

        # Order matters for wasm-merge: a module's imports resolve
        # against the exports of *previous* inputs only.
        argv: list[str] = [
            merge_bin,
            str(runtime_stripped),
            "tcl",
            str(user_stripped),
            "user",
            "--rename-export-conflicts",
            "--all-features",
            "-o",
            str(out_path),
        ]

        proc = subprocess.run(  # noqa: S603 — argv is fully controlled
            argv,
            check=False,
            capture_output=True,
            text=True,
        )
        if proc.returncode != 0:
            msg = f"wasm-merge exited {proc.returncode}\nargv: {argv}\nstderr:\n{proc.stderr}"
            raise WasmMergeFailedError(msg)

        return out_path.read_bytes()

"""End-to-end test: compile + link + run a Tcl script via the WASM CLI.

Drives the real ``explorer.wasm_cli`` entry point (the same code the
``tcl-wasm`` zipapp ships) with ``--link`` to produce a self-contained
``.wasm``, then instantiates that binary under ``wasmtime`` and asserts
the program actually executes — ``puts hello`` must print ``hello``.

This is the integration counterpart to ``test_wasm_bundle.py`` (which
exercises the library function directly): here we go through the CLI's
argument parsing and linked-binary output path, so a regression in the
CLI wiring or the bundled-runtime resolution is caught.

Skipped when the Binaryen tools or the runtime artifact are missing —
those are environment prerequisites, not unit-test concerns.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from compiler.codegen.wasm.extensions import default_runtime_path  # noqa: E402

_REPO_ROOT = Path(__file__).resolve().parents[1]


pytestmark = [
    pytest.mark.skipif(
        shutil.which("wasm-merge") is None or shutil.which("wasm-opt") is None,
        reason="Binaryen (wasm-merge/wasm-opt) not installed",
    ),
    pytest.mark.skipif(
        not default_runtime_path().exists(),
        reason="Runtime artifact not built — run `cd runtime/zig && zig build`",
    ),
]


def _run_linked_wasm(wasm_path: Path) -> tuple[str, str]:
    """Instantiate *wasm_path* under wasmtime, run it, return (out, err).

    The runtime always declares ``env.host_spawn`` /
    ``env.call_compiled_proc``; neither is exercised by ``puts hello``,
    so both are stubbed to trap loudly if the program unexpectedly
    reaches them.
    """
    import os  # noqa: PLC0415

    fd_out, stdout_path = tempfile.mkstemp(suffix=".out")
    fd_err, stderr_path = tempfile.mkstemp(suffix=".err")
    os.close(fd_out)
    os.close(fd_err)
    try:
        engine = wasmtime.Engine()
        store = wasmtime.Store(engine)

        cfg = wasmtime.WasiConfig()
        cfg.stdout_file = stdout_path
        cfg.stderr_file = stderr_path
        store.set_wasi(cfg)

        linker = wasmtime.Linker(engine)
        linker.define_wasi()

        def _trap(*args: int) -> int:
            msg = f"env.* import unexpectedly called: args={args}"
            raise RuntimeError(msg)

        i32 = wasmtime.ValType.i32()
        sig4_to_i32 = wasmtime.FuncType([i32, i32, i32, i32], [i32])
        linker.define_func("env", "call_compiled_proc", sig4_to_i32, _trap)
        linker.define_func("env", "host_spawn", sig4_to_i32, _trap)

        module = wasmtime.Module(engine, wasm_path.read_bytes())
        instance = linker.instantiate(store, module)

        exports = instance.exports(store)
        init_fn = exports.get("_initialize")
        if init_fn is not None:
            init_fn(store)
        top_fn = exports.get("::top")
        assert top_fn is not None, "linked module is missing the ::top entry point"
        top_fn(store)

        return Path(stdout_path).read_text(), Path(stderr_path).read_text()
    finally:
        Path(stdout_path).unlink(missing_ok=True)
        Path(stderr_path).unlink(missing_ok=True)


def test_cli_link_compiles_and_runs_puts_hello(tmp_path: Path) -> None:
    """``wasm_cli --link`` output of ``puts hello`` runs and prints hello."""
    out_wasm = tmp_path / "hello.wasm"
    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "explorer.wasm_cli",
            "--source",
            "puts hello",
            "--link",
            "-o",
            str(out_wasm),
        ],
        cwd=str(_REPO_ROOT),
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, f"CLI failed: stdout={proc.stdout!r} stderr={proc.stderr!r}"
    assert out_wasm.is_file(), "CLI did not write the linked .wasm"

    stdout, stderr = _run_linked_wasm(out_wasm)
    assert stdout == "hello\n", (stdout, stderr)
    assert stderr == "", stderr

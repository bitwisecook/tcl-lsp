"""End-to-end smoke tests for the post-codegen WASM bundler.

Verifies that :func:`wasm_link_bundled` produces a self-contained
``.wasm`` (no host-side runtime re-exporting needed) and that the
extension-manifest plumbing routes ``package require Tcltest`` to the
tcltest-enabled runtime variant.

Skipped when the runtime artifacts haven't been built or when
Binaryen ``wasm-merge`` isn't on ``PATH`` — those are environment
prerequisites, not unit-test concerns.
"""

from __future__ import annotations

import shutil
import tempfile
from pathlib import Path

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from compiler.codegen.wasm.extensions import (  # noqa: E402
    _tcltest_runtime_path,
    default_runtime_path,
)
from compiler.codegen.wasm.link import wasm_link_bundled  # noqa: E402

_REPO_ROOT = Path(__file__).resolve().parents[1]
_SAMPLES = _REPO_ROOT / "samples"


def _have_wasm_merge() -> bool:
    return shutil.which("wasm-merge") is not None


def _have_runtime_variants() -> bool:
    return default_runtime_path().exists() and _tcltest_runtime_path().exists()


pytestmark = [
    pytest.mark.skipif(not _have_wasm_merge(), reason="wasm-merge (Binaryen) not installed"),
    pytest.mark.skipif(
        not _have_runtime_variants(),
        reason=(
            "Runtime variants not built — run `cd runtime/zig && zig build` "
            "to produce both tcl_runtime.wasm and tcl_runtime_with_tcltest.wasm"
        ),
    ),
]


def _instantiate_bundle(bundle_bytes: bytes) -> tuple[str, str]:
    """Instantiate *bundle_bytes* under wasmtime and run ``::top``.

    Returns (stdout, stderr) captured from the WASI sandbox.  ``env``
    imports are stubbed to trap loudly — the bundle should not need
    ``env.call_compiled_proc`` (no compiled procs in the smoke
    samples) or ``env.host_spawn`` (no ``exec``).
    """
    import os  # noqa: PLC0415

    fd_out, stdout_path = tempfile.mkstemp(suffix=".out")
    fd_err, stderr_path = tempfile.mkstemp(suffix=".err")
    os.close(fd_out)
    os.close(fd_err)

    # try/finally so an unexpected wasmtime exception (or a
    # ``RuntimeError`` from ``_trap``) doesn't leak the temp files
    # — Copilot review, PR #363.
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
            msg = f"env.* imported but not exercised by bundle smoke test: args={args}"
            raise RuntimeError(msg)

        i32 = wasmtime.ValType.i32()
        sig4_to_i32 = wasmtime.FuncType([i32, i32, i32, i32], [i32])
        linker.define_func("env", "call_compiled_proc", sig4_to_i32, _trap)
        linker.define_func("env", "host_spawn", sig4_to_i32, _trap)

        module = wasmtime.Module(engine, bundle_bytes)
        instance = linker.instantiate(store, module)

        init_fn = instance.exports(store).get("_initialize")
        if init_fn is not None:
            init_fn(store)

        top_fn = instance.exports(store).get("::top")
        if top_fn is not None:
            try:
                top_fn(store)
            except wasmtime.Trap:
                # Caller may legitimately want to assert on stderr
                # (the "invalid command name" path traps).
                pass

        stdout = Path(stdout_path).read_text()
        stderr = Path(stderr_path).read_text()
        return stdout, stderr
    finally:
        Path(stdout_path).unlink(missing_ok=True)
        Path(stderr_path).unlink(missing_ok=True)


def test_lean_bundle_runs_without_extensions(tmp_path: Path) -> None:
    """Bundles without ``package require Tcltest`` use the lean runtime."""
    src = tmp_path / "hello.tcl"
    src.write_text("puts hello\n")

    bundle_bytes = wasm_link_bundled(src)
    stdout, stderr = _instantiate_bundle(bundle_bytes)
    assert stdout == "hello\n", (stdout, stderr)
    assert stderr == "", stderr


def test_tcltest_bundle_runs_test_commands() -> None:
    """``package require Tcltest`` triggers the tcltest runtime variant."""
    bundle_bytes = wasm_link_bundled(_SAMPLES / "tcltest_smoke.tcl")
    stdout, stderr = _instantiate_bundle(bundle_bytes)
    assert stdout == "42\n70\n1\n", (stdout, stderr)
    assert stderr == "", stderr


def test_lean_bundle_rejects_test_commands(tmp_path: Path) -> None:
    """Calling tcltest commands without ``package require`` traps."""
    src = tmp_path / "no_require.tcl"
    src.write_text("testintobj set 0 42\n")

    bundle_bytes = wasm_link_bundled(src)
    _stdout, stderr = _instantiate_bundle(bundle_bytes)
    assert 'invalid command name "testintobj"' in stderr, stderr

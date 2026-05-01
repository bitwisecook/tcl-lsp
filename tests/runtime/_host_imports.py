"""Shared host-import helpers for runtime unit tests.

The Zig runtime declares two ``env`` imports — ``call_compiled_proc``
(used by the interpreter to dispatch into a compiled module) and
``host_spawn`` (used by the capability-gated ``exec`` BUILTIN).
Every runtime test that instantiates the wasm module needs to
satisfy them; this helper centralises the boilerplate so individual
tests don't drift out of sync when the import surface changes.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")


def define_host_spawn(linker: "wasmtime.Linker", *, callback=None) -> None:
    """Register ``env.host_spawn`` with a default trapping stub.

    Runtime tests don't normally exercise the ``exec`` path, so the
    default callback raises a ``RuntimeError`` if it's reached — that
    surfaces a wiring bug instead of silently returning empty
    strings.  Tests that intentionally drive ``exec`` pass a real
    callable through *callback*; the host-side argv decoding lives
    in the larger ``tests/test_wasm_real_tcl.py`` harness.
    """

    def _call(argv_ptr: int, argv_len: int, stdin_ptr: int, stdin_len: int) -> int:
        if callback is None:
            raise RuntimeError("host_spawn called from a runtime unit test that did not opt in")
        return callback(argv_ptr, argv_len, stdin_ptr, stdin_len)

    try:
        linker.define_func(
            "env",
            "host_spawn",
            wasmtime.FuncType(
                [
                    wasmtime.ValType.i32(),
                    wasmtime.ValType.i32(),
                    wasmtime.ValType.i32(),
                    wasmtime.ValType.i32(),
                ],
                [wasmtime.ValType.i32()],
            ),
            _call,
        )
    except wasmtime.WasmtimeError:
        # Linker rejects redefinitions inside one Linker; matches
        # the existing pattern used by other env imports.
        pass

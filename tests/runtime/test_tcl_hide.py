"""Unit tests for ``tcl_hide.hide_command`` / ``tcl_hide.expose_command``.

Drives the runtime's hide/expose primitives directly through the
``tcl_test_hide`` / ``tcl_test_expose`` exports so we can assert
interpreter-wide hidden-table state before and after the call
without routing through the full Tcl interpreter.

See ``docs/design/runtime/command-introspection.md`` for the
contract + ``runtime/zig/tcl_hide.zig`` for the per-case comments.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")


from core.runtime_wasm import runtime_wasm_path  # noqa: E402

_ZIG_RUNTIME_PATH = runtime_wasm_path()


# Match the ``HideResult`` / ``ExposeResult`` enums in tcl_hide.zig.
HIDE_OK = 0
HIDE_NOT_FOUND = 1
HIDE_QUALIFIED_REJECTED = 2
HIDE_NAME_TAKEN = 3

EXPOSE_OK = 0
EXPOSE_NOT_FOUND = 1
EXPOSE_QUALIFIED_REJECTED = 2
EXPOSE_TARGET_EXISTS = 3


@pytest.fixture()
def runtime():
    if not _ZIG_RUNTIME_PATH.exists():
        pytest.skip(f"Zig WASM runtime not built: {_ZIG_RUNTIME_PATH}")

    engine = wasmtime.Engine()
    store = wasmtime.Store(engine)
    store.set_wasi(wasmtime.WasiConfig())
    linker = wasmtime.Linker(engine)
    linker.define_wasi()

    def _unused_call(name_ptr: int, name_len: int, argv: int, argc: int) -> int:
        raise RuntimeError("call_compiled_proc: unexpected call from hide test")

    linker.define_func(
        "env",
        "call_compiled_proc",
        wasmtime.FuncType(
            [
                wasmtime.ValType.i32(),
                wasmtime.ValType.i32(),
                wasmtime.ValType.i32(),
                wasmtime.ValType.i32(),
            ],
            [wasmtime.ValType.i32()],
        ),
        _unused_call,
    )

    module = wasmtime.Module.from_file(engine, str(_ZIG_RUNTIME_PATH))
    instance = linker.instantiate(store, module)
    return RuntimeHandle(store, instance)


class RuntimeHandle:
    def __init__(self, store: wasmtime.Store, instance: wasmtime.Instance):
        self.store = store
        self.instance = instance
        exports = instance.exports(store)
        self.memory: wasmtime.Memory = exports["memory"]
        self.test_alloc = exports["tcl_test_alloc"]
        self.ns_root = exports["tcl_ns_root"]
        self.ns_create = exports["tcl_ns_create"]
        self.obj_new_string = exports["obj_new_string"]
        self.proc_register = exports["proc_register"]
        self.proc_exists = exports["proc_exists"]
        self.obj_get_int = exports["obj_get_int"]
        self.test_hide = exports["tcl_test_hide"]
        self.test_expose = exports["tcl_test_expose"]
        self.hidden_exists = exports["tcl_test_hidden_exists"]

    def stage(self, s: str) -> tuple[int, int]:
        encoded = s.encode("utf-8")
        addr = self.test_alloc(self.store, len(encoded))
        self.memory.write(self.store, encoded, addr)
        return addr, len(encoded)

    def root(self) -> int:
        return self.ns_root(self.store)

    def register_proc(self, fqn: str) -> None:
        ptr, length = self.stage(fqn)
        name_obj = self.obj_new_string(self.store, ptr, length)
        pptr, plen = self.stage("")
        params_obj = self.obj_new_string(self.store, pptr, plen)
        bptr, blen = self.stage("")
        body_obj = self.obj_new_string(self.store, bptr, blen)
        self.proc_register(self.store, name_obj, params_obj, body_obj)

    def exists(self, fqn: str) -> bool:
        ptr, length = self.stage(fqn)
        name_obj = self.obj_new_string(self.store, ptr, length)
        result_obj = self.proc_exists(self.store, name_obj)
        return self.obj_get_int(self.store, result_obj) != 0

    def is_hidden(self, name: str) -> bool:
        ptr, length = self.stage(name)
        return self.hidden_exists(self.store, ptr, length) != 0

    def hide(self, src_ns: int, src_simple: str, hidden_name: str) -> int:
        sp, sl = self.stage(src_simple)
        hp, hl = self.stage(hidden_name)
        return self.test_hide(self.store, src_ns, sp, sl, hp, hl)

    def expose(self, hidden_name: str, dest_ns: int, new_simple: str) -> int:
        hp, hl = self.stage(hidden_name)
        np, nl = self.stage(new_simple)
        return self.test_expose(self.store, hp, hl, dest_ns, np, nl)


def test_hide_basic(runtime: RuntimeHandle):
    """Hiding a proc removes it from its ns's cmd_table and stashes
    it under the given name in the hidden table."""
    root = runtime.root()
    runtime.register_proc("::foo")
    assert runtime.exists("::foo")
    assert not runtime.is_hidden("foo")

    r = runtime.hide(root, "foo", "foo")
    assert r == HIDE_OK
    assert not runtime.exists("::foo")
    assert runtime.is_hidden("foo")


def test_hide_with_different_hidden_name(runtime: RuntimeHandle):
    """Hidden name can differ from the source simple name."""
    root = runtime.root()
    runtime.register_proc("::bar")
    r = runtime.hide(root, "bar", "secret")
    assert r == HIDE_OK
    assert not runtime.exists("::bar")
    assert runtime.is_hidden("secret")
    assert not runtime.is_hidden("bar")


def test_hide_not_found(runtime: RuntimeHandle):
    """Hiding a command that doesn't exist returns not_found."""
    root = runtime.root()
    r = runtime.hide(root, "nope", "nope")
    assert r == HIDE_NOT_FOUND


def test_hide_rejects_qualified_source(runtime: RuntimeHandle):
    """Qualified source names are not accepted."""
    root = runtime.root()
    runtime.register_proc("::foo")
    r = runtime.hide(root, "::foo", "foo")
    assert r == HIDE_QUALIFIED_REJECTED


def test_hide_rejects_qualified_hidden_name(runtime: RuntimeHandle):
    """Qualified hidden names are not accepted — hidden commands
    always live at the top of the interpreter-wide hidden table."""
    root = runtime.root()
    runtime.register_proc("::foo")
    r = runtime.hide(root, "foo", "::secret")
    assert r == HIDE_QUALIFIED_REJECTED


def test_hide_name_already_taken(runtime: RuntimeHandle):
    """Hiding under a name already present in the hidden table fails."""
    root = runtime.root()
    runtime.register_proc("::first")
    runtime.register_proc("::second")
    assert runtime.hide(root, "first", "shared") == HIDE_OK
    assert runtime.hide(root, "second", "shared") == HIDE_NAME_TAKEN
    # ``second`` still lives in the cmd_table — the second hide was
    # a no-op.
    assert runtime.exists("::second")


def test_expose_basic(runtime: RuntimeHandle):
    """Exposing brings a hidden command back into the current ns."""
    root = runtime.root()
    runtime.register_proc("::foo")
    assert runtime.hide(root, "foo", "foo") == HIDE_OK

    r = runtime.expose("foo", root, "foo")
    assert r == EXPOSE_OK
    assert runtime.exists("::foo")
    assert not runtime.is_hidden("foo")


def test_expose_with_different_destination(runtime: RuntimeHandle):
    """Expose can rename during the move — ``interp expose {}
    hidden newName`` lands the entry under ``newName``."""
    root = runtime.root()
    runtime.register_proc("::foo")
    assert runtime.hide(root, "foo", "foo") == HIDE_OK

    r = runtime.expose("foo", root, "rescued")
    assert r == EXPOSE_OK
    assert runtime.exists("::rescued")
    assert not runtime.is_hidden("foo")


def test_expose_not_found(runtime: RuntimeHandle):
    """Exposing a name that isn't in the hidden table fails."""
    root = runtime.root()
    r = runtime.expose("nope", root, "nope")
    assert r == EXPOSE_NOT_FOUND


def test_expose_target_exists(runtime: RuntimeHandle):
    """Exposing into a live cmd_table slot is rejected."""
    root = runtime.root()
    runtime.register_proc("::foo")
    runtime.register_proc("::bar")
    assert runtime.hide(root, "foo", "foo") == HIDE_OK

    # ``bar`` is live — can't expose ``foo`` as ``bar``.
    r = runtime.expose("foo", root, "bar")
    assert r == EXPOSE_TARGET_EXISTS
    # ``foo`` is still hidden, ``bar`` still live.
    assert runtime.is_hidden("foo")
    assert runtime.exists("::bar")


def test_expose_rejects_qualified_destination(runtime: RuntimeHandle):
    """Exposed names must be simple (live in the current ns)."""
    root = runtime.root()
    runtime.register_proc("::foo")
    assert runtime.hide(root, "foo", "foo") == HIDE_OK

    r = runtime.expose("foo", root, "::ns::rescued")
    assert r == EXPOSE_QUALIFIED_REJECTED


def test_hide_then_expose_round_trip(runtime: RuntimeHandle):
    """Hide + expose restores the command to a callable state."""
    root = runtime.root()
    runtime.register_proc("::cycle")
    assert runtime.hide(root, "cycle", "cycle") == HIDE_OK
    assert not runtime.exists("::cycle")
    assert runtime.expose("cycle", root, "cycle") == EXPOSE_OK
    assert runtime.exists("::cycle")

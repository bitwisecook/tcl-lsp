"""Unit tests for ``tcl_rename.rename_command`` (R1 — R5).

Drives the runtime's rename primitive directly through the
``tcl_test_rename`` / ``tcl_test_alias_create`` exports so we can
assert namespace-tree state before and after the call without
routing through the full Tcl interpreter.

See ``docs/design/runtime/rename-alias.md`` for the contract +
``runtime/zig/tcl_rename.zig`` for the per-case comments.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from core.runtime_wasm import runtime_wasm_path  # noqa: E402
from tests.runtime._host_imports import define_host_spawn  # noqa: E402

_ZIG_RUNTIME_PATH = runtime_wasm_path()


# Match the ``RenameResult`` enum in tcl_rename.zig.
OK = 0
NOT_FOUND = 1
TARGET_EXISTS = 2
BUILTIN_PROTECTED = 3


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
        raise RuntimeError("call_compiled_proc: unexpected call from rename test")

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
    define_host_spawn(linker)
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
        self.test_rename = exports["tcl_test_rename"]
        self.obj_new_string = exports["obj_new_string"]
        self.proc_register = exports["proc_register"]
        self.proc_exists = exports["proc_exists"]
        self.proc_lookup = exports["proc_lookup"]
        self.proc_get_name_ptr = exports["proc_get_name_ptr"]
        self.proc_get_name_len = exports["proc_get_name_len"]
        self.obj_get_int = exports["obj_get_int"]

    def stage(self, s: str) -> tuple[int, int]:
        encoded = s.encode("utf-8")
        addr = self.test_alloc(self.store, len(encoded))
        self.memory.write(self.store, encoded, addr)
        return addr, len(encoded)

    def read(self, ptr: int, length: int) -> bytes:
        if length == 0:
            return b""
        return bytes(self.memory.read(self.store, ptr, ptr + length))

    def root(self) -> int:
        return self.ns_root(self.store)

    def create(self, parent: int, name: str) -> int:
        ptr, length = self.stage(name)
        return self.ns_create(self.store, parent, ptr, length)

    def new_string(self, s: str) -> int:
        ptr, length = self.stage(s)
        return self.obj_new_string(self.store, ptr, length)

    def register_proc(self, fqn: str, params: str = "", body: str = "") -> None:
        name_obj = self.new_string(fqn)
        params_obj = self.new_string(params)
        body_obj = self.new_string(body)
        self.proc_register(self.store, name_obj, params_obj, body_obj)

    def rename(
        self,
        old_ns: int,
        old_simple: str,
        new_ns: int,
        new_simple: str,
    ) -> int:
        op, ol = self.stage(old_simple)
        np, nl = self.stage(new_simple)
        return self.test_rename(self.store, old_ns, op, ol, new_ns, np, nl)

    def exists(self, fqn: str) -> bool:
        # ``proc_exists`` returns a TclObj (pointer) holding 0/1, not
        # a raw boolean — unwrap via ``obj_get_int``.
        obj = self.new_string(fqn)
        result_obj = self.proc_exists(self.store, obj)
        return self.obj_get_int(self.store, result_obj) != 0

    def lookup_name(self, fqn: str) -> str | None:
        """Resolve ``fqn`` to a Command; return the Command's stored
        FQN (or None on miss)."""
        obj = self.new_string(fqn)
        cmd = self.proc_lookup(self.store, obj)
        if not cmd:
            return None
        name_ptr = self.proc_get_name_ptr(self.store, cmd)
        name_len = self.proc_get_name_len(self.store, cmd)
        return self.read(name_ptr, name_len).decode("utf-8")


# -- R1: simple same-ns rename ----------------------------------------------


def test_rename_simple_same_ns(runtime: RuntimeHandle):
    """Rename a proc from ``foo`` to ``bar`` in the root namespace.
    Old name stops resolving, new name resolves, Command's FQN slot
    updates to match the new name."""
    root = runtime.root()
    runtime.register_proc("::foo")
    assert runtime.exists("::foo")
    assert not runtime.exists("::bar")

    r = runtime.rename(root, "foo", root, "bar")
    assert r == OK
    assert not runtime.exists("::foo")
    assert runtime.exists("::bar")
    # Command's own FQN slot was patched.
    assert runtime.lookup_name("::bar") == "::bar"


def test_rename_self_is_noop(runtime: RuntimeHandle):
    """``rename foo foo`` succeeds without touching anything."""
    root = runtime.root()
    runtime.register_proc("::foo")
    r = runtime.rename(root, "foo", root, "foo")
    assert r == OK
    assert runtime.exists("::foo")


def test_rename_source_missing(runtime: RuntimeHandle):
    """Rename of a non-existent command returns ``not_found``."""
    root = runtime.root()
    r = runtime.rename(root, "nope", root, "somewhere")
    assert r == NOT_FOUND


def test_rename_target_occupied(runtime: RuntimeHandle):
    """Rename to an already-used name returns ``target_exists``."""
    root = runtime.root()
    runtime.register_proc("::foo")
    runtime.register_proc("::bar")
    r = runtime.rename(root, "foo", root, "bar")
    assert r == TARGET_EXISTS
    # Both still exist.
    assert runtime.exists("::foo")
    assert runtime.exists("::bar")


def test_rename_builtin_protected(runtime: RuntimeHandle):
    """The ``return`` / ``error`` hardcoded list refuses rename."""
    root = runtime.root()
    # Create a shadow proc under ``return`` so the source lookup
    # succeeds — the protection check fires regardless.
    runtime.register_proc("::return")
    r = runtime.rename(root, "return", root, "ret2")
    assert r == BUILTIN_PROTECTED


# -- R2: cross-namespace rename ---------------------------------------------


def test_rename_across_namespaces(runtime: RuntimeHandle):
    """Rename moves ``::foo`` to ``::ns::bar``: old resolves to 0,
    new resolves to the same Command, FQN updates accordingly."""
    root = runtime.root()
    ns = runtime.create(root, "ns")
    runtime.register_proc("::foo")

    # Capture the Command pointer before the move.
    old_cmd = runtime.proc_lookup(runtime.store, runtime.new_string("::foo"))
    assert old_cmd != 0

    r = runtime.rename(root, "foo", ns, "bar")
    assert r == OK
    assert not runtime.exists("::foo")
    assert runtime.exists("::ns::bar")
    # Same Command pointer — only the hash key + FQN slot moved.
    new_cmd = runtime.proc_lookup(runtime.store, runtime.new_string("::ns::bar"))
    assert new_cmd == old_cmd
    assert runtime.lookup_name("::ns::bar") == "::ns::bar"


# -- R5: delete via rename-to-empty -----------------------------------------


def test_rename_to_empty_deletes(runtime: RuntimeHandle):
    """``rename foo {}`` clears the source slot so lookups miss."""
    root = runtime.root()
    runtime.register_proc("::foo")
    r = runtime.rename(root, "foo", 0, "")
    assert r == OK
    assert not runtime.exists("::foo")


def test_rename_to_empty_then_recreate(runtime: RuntimeHandle):
    """After delete, registering again puts a fresh command in the
    same slot (the tombstoned bucket reuses successfully)."""
    root = runtime.root()
    runtime.register_proc("::foo", "", "body1")
    r = runtime.rename(root, "foo", 0, "")
    assert r == OK
    runtime.register_proc("::foo", "", "body2")
    assert runtime.exists("::foo")


# -- P1: compiled-proc rename preserves the sidecar export name -------------


def test_rename_compiled_proc_preserves_export_name(runtime: RuntimeHandle):
    """Renaming a compiled proc rewrites the Command's live name
    slot but keeps the sidecar ``OFF_EXPORT_NAME_BUCKET`` pointing
    at the registration-time export name.  The host-bridge dispatch
    consumes the sidecar, so cross-module calls survive the
    rename."""
    exports = runtime.instance.exports(runtime.store)
    proc_register_compiled = exports["proc_register_compiled"]
    export_name_ptr = exports["tcl_test_export_name_ptr"]
    export_name_len = exports["tcl_test_export_name_len"]
    proc_lookup = exports["proc_lookup"]

    root = runtime.root()
    name_obj = runtime.new_string("::compiled_proc")
    proc_register_compiled(runtime.store, name_obj, 0, 7, 0, 0)

    # Sidecar captures the registration-time FQN.
    bucket = proc_lookup(runtime.store, runtime.new_string("::compiled_proc"))
    assert bucket != 0
    before_ptr = export_name_ptr(runtime.store, bucket)
    before_len = export_name_len(runtime.store, bucket)
    assert before_len == len("::compiled_proc")
    assert runtime.read(before_ptr, before_len).decode("utf-8") == "::compiled_proc"

    # Renaming flips the Command's live name slot; the sidecar
    # stays untouched.
    r = runtime.rename(root, "compiled_proc", root, "renamed_compiled")
    assert r == OK

    bucket = proc_lookup(runtime.store, runtime.new_string("::renamed_compiled"))
    assert bucket != 0
    # Live name slot reflects the rename.
    assert runtime.lookup_name("::renamed_compiled") == "::renamed_compiled"
    # Sidecar still points at the original export name.
    after_ptr = export_name_ptr(runtime.store, bucket)
    after_len = export_name_len(runtime.store, bucket)
    assert after_len == before_len
    assert runtime.read(after_ptr, after_len).decode("utf-8") == "::compiled_proc"

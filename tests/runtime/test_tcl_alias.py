"""Unit tests for ``tcl_alias`` (A1 — A4).

Creates aliases directly via the ``tcl_test_alias_create`` export so
the assertions don't route through the full ``interp alias`` parser.
Dispatch is exercised through the interpreter by staging + calling
``tcl_eval`` on a short script containing a call to the alias.
"""

from __future__ import annotations

from pathlib import Path

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")


from core.runtime_wasm import runtime_wasm_path

_ZIG_RUNTIME_PATH = runtime_wasm_path()


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
        raise RuntimeError("call_compiled_proc: unexpected call from alias test")

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
        self.test_alias_create = exports["tcl_test_alias_create"]
        self.test_rename = exports["tcl_test_rename"]
        self.obj_new_string = exports["obj_new_string"]
        self.obj_get_int = exports["obj_get_int"]
        self.proc_register = exports["proc_register"]
        self.proc_lookup = exports["proc_lookup"]
        self.proc_exists = exports["proc_exists"]
        self.tcl_eval = exports["tcl_eval"]

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

    def new_string(self, s: str) -> int:
        ptr, length = self.stage(s)
        return self.obj_new_string(self.store, ptr, length)

    def register_proc(self, fqn: str, params: str, body: str) -> None:
        name_obj = self.new_string(fqn)
        params_obj = self.new_string(params)
        body_obj = self.new_string(body)
        self.proc_register(self.store, name_obj, params_obj, body_obj)

    def exists(self, fqn: str) -> bool:
        obj = self.new_string(fqn)
        result = self.proc_exists(self.store, obj)
        return self.obj_get_int(self.store, result) != 0

    def create_alias(
        self,
        dest_ns: int,
        new_simple: str,
        target_fqn: str,
        prefix: list[str] | None = None,
    ) -> int:
        np, nl = self.stage(new_simple)
        tp, tl = self.stage(target_fqn)
        prefix = prefix or []
        n_prefix = len(prefix)
        prefix_buf = 0
        if n_prefix > 0:
            prefix_buf = self.test_alloc(self.store, n_prefix * 4)
            for i, s in enumerate(prefix):
                obj = self.new_string(s)
                # Pack the TclObj handle (i32) into the u32 array.
                self.memory.write(
                    self.store,
                    int(obj).to_bytes(4, "little", signed=True),
                    prefix_buf + i * 4,
                )
        return self.test_alias_create(self.store, dest_ns, np, nl, tp, tl, n_prefix, prefix_buf)

    def eval_script(self, script: str) -> int:
        script_obj = self.new_string(script)
        return self.tcl_eval(self.store, script_obj)


# TclObj layout: type_tag=4, int_cache=8, str_ptr=16, str_len=20 (see
# ``OBJ_STR_PTR`` / ``OBJ_STR_LEN`` in tcl_obj.zig).  Obj handle 0
# represents an empty string.
def _obj_str(runtime: RuntimeHandle, obj: int) -> tuple[int, int]:
    if obj == 0:
        return 0, 0
    ptr = int.from_bytes(
        runtime.memory.read(runtime.store, obj + 16, obj + 20),
        "little",
        signed=True,
    )
    length = int.from_bytes(
        runtime.memory.read(runtime.store, obj + 20, obj + 24),
        "little",
        signed=True,
    )
    return ptr, length


# -- A1: simple alias, no prefix --------------------------------------------


def test_alias_no_prefix_dispatches(runtime: RuntimeHandle):
    """Define ``proc ::orig {x} {return $x}``, alias ``::a → ::orig``,
    verify ``a 42`` returns ``42``."""
    runtime.register_proc("::orig", "x", "set ::result $x")
    root = runtime.root()
    runtime.create_alias(root, "a", "::orig")
    assert runtime.exists("::a")

    runtime.eval_script("a hello")
    # Read the global ::result that orig wrote.
    result_obj = runtime.new_string("::result")
    # Use global_get to fetch the value; unwrap via ``_obj_str``
    # which reads ``OBJ_STR_PTR`` (offset 16) / ``OBJ_STR_LEN``
    # (offset 20) — see ``tcl_obj.zig`` for the TclObj layout.
    global_get = runtime.instance.exports(runtime.store)["global_get"]
    val_obj = global_get(runtime.store, result_obj)
    ptr, length = _obj_str(runtime, val_obj)
    assert runtime.read(ptr, length) == b"hello"


# -- A2: alias with prefix args ---------------------------------------------


def test_alias_prefix_prepended(runtime: RuntimeHandle):
    """Define ``proc ::two {a b} {set ::result "$a-$b"}``,
    alias ``::a → ::two prefix``, verify ``a tail`` runs as
    ``two prefix tail``."""
    runtime.register_proc("::two", "a b", 'set ::result "$a-$b"')
    root = runtime.root()
    runtime.create_alias(root, "a", "::two", prefix=["head"])
    runtime.eval_script("a tail")

    global_get = runtime.instance.exports(runtime.store)["global_get"]
    val_obj = global_get(runtime.store, runtime.new_string("::result"))
    ptr, length = _obj_str(runtime, val_obj)
    assert runtime.read(ptr, length) == b"head-tail"


# -- A4: alias + rename interaction -----------------------------------------


def test_alias_rename_of_target_breaks_lookup(runtime: RuntimeHandle):
    """Aliases resolve their stored target name on each call.  Because
    resolution is by *string*, renaming the target command makes the
    stored name stop resolving — the alias does NOT automatically
    follow the rename.  If the target is later recreated at the
    original name, the alias starts working again; otherwise calls
    through the alias raise ``unknown command: <target>``.
    """
    runtime.register_proc("::orig", "x", "set ::result called:$x")
    root = runtime.root()
    runtime.create_alias(root, "a", "::orig")
    # Rename ::orig → ::renamed.
    r = runtime.test_rename(
        runtime.store,
        root,
        *runtime.stage("orig"),
        root,
        *runtime.stage("renamed"),
    )
    assert r == 0  # RenameResult.ok
    # Alias redirect is still in the cmd_table; the target name
    # it stores ("::orig") no longer resolves because the Command
    # moved to the cmd_table bucket keyed on "renamed".
    assert runtime.exists("::a")
    assert not runtime.exists("::orig")
    assert runtime.exists("::renamed")


def test_alias_rename_retargets_via_create(runtime: RuntimeHandle):
    """Redefining the alias under a new target name takes effect
    immediately — the cmd_table slot is replaced."""
    runtime.register_proc("::a_target", "x", "set ::result A:$x")
    runtime.register_proc("::b_target", "x", "set ::result B:$x")
    root = runtime.root()
    runtime.create_alias(root, "a", "::a_target")
    runtime.eval_script("a hello")
    global_get = runtime.instance.exports(runtime.store)["global_get"]
    r1 = global_get(runtime.store, runtime.new_string("::result"))
    ptr, length = _obj_str(runtime, r1)
    assert runtime.read(ptr, length) == b"A:hello"

    # Replace the alias.
    runtime.create_alias(root, "a", "::b_target")
    runtime.eval_script("a world")
    r2 = global_get(runtime.store, runtime.new_string("::result"))
    ptr, length = _obj_str(runtime, r2)
    assert runtime.read(ptr, length) == b"B:world"

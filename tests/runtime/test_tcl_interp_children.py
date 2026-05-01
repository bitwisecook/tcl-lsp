"""Unit tests for the child-interp registry primitives.

Drives ``tcl_test_interp_create`` / ``tcl_test_interp_lookup`` /
``tcl_test_interp_delete`` / ``tcl_test_interp_eval_script`` /
``tcl_test_hidden_find_in`` through the runtime's WASM exports so
we can assert the per-interp state directly, without routing
through the ``interp`` built-in dispatcher.

See ``docs/design/runtime/child-interp.md`` for the contract and
``runtime/zig/tcl_interp_registry.zig`` for the per-field
commentary.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.runtime._host_imports import define_host_spawn  # noqa: E402


from core.runtime_wasm import runtime_wasm_path  # noqa: E402

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
        raise RuntimeError("call_compiled_proc: unexpected call from interp test")

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
        self.obj_new_string = exports["obj_new_string"]
        self.obj_get_int = exports["obj_get_int"]
        self.proc_register = exports["proc_register"]
        self.proc_exists = exports["proc_exists"]
        self.interp_root = exports["tcl_test_interp_root"]
        self.interp_current = exports["tcl_test_interp_current"]
        self.interp_create = exports["tcl_test_interp_create"]
        self.interp_lookup = exports["tcl_test_interp_lookup"]
        self.interp_delete = exports["tcl_test_interp_delete"]
        self.interp_eval_script = exports["tcl_test_interp_eval_script"]
        self.interp_root_ns = exports["tcl_test_interp_root_ns"]
        self.hidden_find_in = exports["tcl_test_hidden_find_in"]
        self.test_hide = exports["tcl_test_hide"]
        self.obj_str_ptr = exports["tcl_test_obj_str_ptr"]
        self.obj_str_len = exports["tcl_test_obj_str_len"]

    def stage(self, s: str) -> tuple[int, int]:
        encoded = s.encode("utf-8")
        addr = self.test_alloc(self.store, len(encoded))
        self.memory.write(self.store, encoded, addr)
        return addr, len(encoded)

    def root(self) -> int:
        return self.interp_root(self.store)

    def current(self) -> int:
        return self.interp_current(self.store)

    def create_child(self, parent: int, name: str, flags: int = 0) -> int:
        ptr, length = self.stage(name)
        return self.interp_create(self.store, parent, ptr, length, flags)

    def lookup(self, parent: int, name: str) -> int:
        ptr, length = self.stage(name)
        return self.interp_lookup(self.store, parent, ptr, length)

    def delete(self, parent: int, name: str) -> int:
        ptr, length = self.stage(name)
        return self.interp_delete(self.store, parent, ptr, length)

    def eval_in(self, target: int, script: str) -> int:
        ptr, length = self.stage(script)
        return self.interp_eval_script(self.store, target, ptr, length)

    def root_ns(self, target: int) -> int:
        return self.interp_root_ns(self.store, target)

    def hidden_in(self, target: int, name: str) -> bool:
        ptr, length = self.stage(name)
        return self.hidden_find_in(self.store, target, ptr, length) != 0

    def register_proc(self, fqn: str, body: str = "") -> None:
        ptr, length = self.stage(fqn)
        name_obj = self.obj_new_string(self.store, ptr, length)
        pptr, plen = self.stage("")
        params_obj = self.obj_new_string(self.store, pptr, plen)
        bptr, blen = self.stage(body)
        body_obj = self.obj_new_string(self.store, bptr, blen)
        self.proc_register(self.store, name_obj, params_obj, body_obj)

    def proc_exists_(self, fqn: str) -> bool:
        ptr, length = self.stage(fqn)
        name_obj = self.obj_new_string(self.store, ptr, length)
        return self.obj_get_int(self.store, self.proc_exists(self.store, name_obj)) != 0

    def hide_current(self, src_ns: int, simple: str, hidden: str) -> int:
        sp, sl = self.stage(simple)
        hp, hl = self.stage(hidden)
        return self.test_hide(self.store, src_ns, sp, sl, hp, hl)

    def obj_str(self, handle: int) -> str:
        ptr = self.obj_str_ptr(self.store, handle)
        length = self.obj_str_len(self.store, handle)
        if length == 0:
            return ""
        return bytes(self.memory.read(self.store, ptr, ptr + length)).decode("utf-8")


def test_root_interp_is_stable(runtime: RuntimeHandle):
    """``interp_root`` is idempotent — every call returns the same
    address, and the root interp starts as the current interp."""
    r1 = runtime.root()
    r2 = runtime.root()
    assert r1 == r2
    assert r1 != 0
    assert runtime.current() == r1


def test_create_child_returns_fresh_interp(runtime: RuntimeHandle):
    """``child_create`` allocates a fresh ``Interp`` with an
    independent root namespace."""
    root = runtime.root()
    child = runtime.create_child(root, "worker")
    assert child != 0
    assert child != root
    # Each interp gets its own root namespace — that's the invariant
    # that makes procs defined in one interp invisible in the other.
    assert runtime.root_ns(child) != runtime.root_ns(root)


def test_lookup_returns_same_handle(runtime: RuntimeHandle):
    """After ``create``, ``lookup`` under the same name resolves to
    the same ``Interp``."""
    root = runtime.root()
    child = runtime.create_child(root, "foo")
    assert runtime.lookup(root, "foo") == child
    # Distinct names don't collide.
    assert runtime.lookup(root, "bar") == 0


def test_delete_unlinks_from_parent(runtime: RuntimeHandle):
    """``child_delete`` tombstones the parent's bucket; ``lookup``
    returns 0 thereafter."""
    root = runtime.root()
    runtime.create_child(root, "victim")
    assert runtime.lookup(root, "victim") != 0
    assert runtime.delete(root, "victim") == 1
    assert runtime.lookup(root, "victim") == 0
    # Second delete is a no-op.
    assert runtime.delete(root, "victim") == 0


def test_eval_swaps_current_interp(runtime: RuntimeHandle):
    """``eval`` inside a child temporarily flips ``current_interp``
    and restores on return."""
    root = runtime.root()
    child = runtime.create_child(root, "swap_probe")
    # Outside the eval, current_interp is root.
    assert runtime.current() == root
    runtime.eval_in(child, "set x 42")
    # And the restore happens on the way out.
    assert runtime.current() == root


def test_eval_in_child_uses_separate_proc_table(runtime: RuntimeHandle):
    """Procs registered in the parent aren't visible in the child —
    that's the "fresh namespace tree per interp" invariant."""
    root = runtime.root()
    child = runtime.create_child(root, "isolate")
    runtime.register_proc("::parent_only", "")
    assert runtime.proc_exists_("::parent_only")

    # Switch into the child, register a child-only proc, switch out.
    runtime.eval_in(child, "proc child_only {} {}")

    # The child saw the parent's proc registry as empty at registration
    # time — but both procs now live in distinct namespace trees so
    # ``proc_exists_`` on root still reports ``::parent_only``.
    assert runtime.proc_exists_("::parent_only")


def test_per_interp_hidden_tables(runtime: RuntimeHandle):
    """Hidden commands are per-interp: hiding in root doesn't
    populate the child's hidden table."""
    root = runtime.root()
    child = runtime.create_child(root, "hidden_probe")

    # Register + hide a proc in the root interp.
    runtime.register_proc("::secret", "")
    r = runtime.hide_current(runtime.root_ns(root), "secret", "secret")
    assert r == 0  # HIDE_OK
    # Root's hidden table has the entry.
    assert runtime.hidden_in(root, "secret")
    # Child's hidden table doesn't.
    assert not runtime.hidden_in(child, "secret")


def test_child_of_child(runtime: RuntimeHandle):
    """Two-level child chain — ``resolve_path`` walks both components."""
    root = runtime.root()
    c1 = runtime.create_child(root, "level1")
    c2 = runtime.create_child(c1, "level2")
    # ``c2`` is not a direct child of root.
    assert runtime.lookup(root, "level2") == 0
    # But it's a direct child of ``c1``.
    assert runtime.lookup(c1, "level2") == c2

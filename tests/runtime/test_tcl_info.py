"""Unit tests for the ``info commands`` / ``info procs`` walker in
``tcl_cmd_info.zig``.

Drives the runtime's info primitives directly through the
``tcl_test_info_commands`` / ``tcl_test_info_procs`` exports so we
can assert the walker's output without routing through the full
interpreter.  ``tcl_test_namespace_which`` covers the companion
``namespace which -command`` probe.
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
        raise RuntimeError("call_compiled_proc: unexpected call from info test")

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
        self.obj_new_string = exports["obj_new_string"]
        self.proc_register = exports["proc_register"]
        self.proc_register_compiled = exports["proc_register_compiled"]
        self.proc_exists = exports["proc_exists"]
        self.obj_get_int = exports["obj_get_int"]
        self.test_info_commands = exports["tcl_test_info_commands"]
        self.test_info_procs = exports["tcl_test_info_procs"]
        self.test_namespace_which = exports["tcl_test_namespace_which"]
        self.test_hide = exports["tcl_test_hide"]

    def stage(self, s: str) -> tuple[int, int]:
        encoded = s.encode("utf-8")
        addr = self.test_alloc(self.store, len(encoded))
        self.memory.write(self.store, encoded, addr)
        return addr, len(encoded)

    def read(self, ptr: int, length: int) -> bytes:
        if length == 0:
            return b""
        return bytes(self.memory.read(self.store, ptr, ptr + length))

    def read_obj_string(self, obj_handle: int) -> str:
        """Read a TclObj's string bytes via its str_ptr / str_len
        slots (offsets 16 / 20 per tcl_obj.zig's OBJ_STR_PTR /
        OBJ_STR_LEN)."""
        if obj_handle == 0:
            return ""
        str_ptr = int.from_bytes(
            self.memory.read(self.store, obj_handle + 16, obj_handle + 20),
            "little",
            signed=True,
        )
        str_len = int.from_bytes(
            self.memory.read(self.store, obj_handle + 20, obj_handle + 24),
            "little",
            signed=True,
        )
        if str_len == 0:
            return ""
        return bytes(self.memory.read(self.store, str_ptr, str_ptr + str_len)).decode("utf-8")

    def root(self) -> int:
        return self.ns_root(self.store)

    def register_proc(self, fqn: str, params: str = "", body: str = " ") -> None:
        ptr, length = self.stage(fqn)
        name_obj = self.obj_new_string(self.store, ptr, length)
        pp, pl = self.stage(params)
        params_obj = self.obj_new_string(self.store, pp, pl)
        bp, bl = self.stage(body)
        body_obj = self.obj_new_string(self.store, bp, bl)
        self.proc_register(self.store, name_obj, params_obj, body_obj)

    def register_compiled(self, fqn: str, func_idx: int = 1) -> None:
        ptr, length = self.stage(fqn)
        name_obj = self.obj_new_string(self.store, ptr, length)
        self.proc_register_compiled(self.store, name_obj, 0, func_idx, 0, 0)

    def info_commands(self, pattern: str = "") -> list[str]:
        if pattern:
            pp, pl = self.stage(pattern)
        else:
            pp, pl = 0, 0
        obj_handle = self.test_info_commands(self.store, pp, pl)
        s = self.read_obj_string(obj_handle)
        return s.split() if s else []

    def info_procs(self, pattern: str = "") -> list[str]:
        if pattern:
            pp, pl = self.stage(pattern)
        else:
            pp, pl = 0, 0
        obj_handle = self.test_info_procs(self.store, pp, pl)
        s = self.read_obj_string(obj_handle)
        return s.split() if s else []

    def namespace_which(self, name: str) -> str:
        pp, pl = self.stage(name)
        obj_handle = self.test_namespace_which(self.store, pp, pl)
        return self.read_obj_string(obj_handle)

    def hide(self, src_ns: int, simple: str, hidden_name: str) -> int:
        sp, sl = self.stage(simple)
        hp, hl = self.stage(hidden_name)
        return self.test_hide(self.store, src_ns, sp, sl, hp, hl)


def test_info_commands_lists_registered_procs(runtime: RuntimeHandle):
    """``info commands`` with no pattern returns every live command
    reachable from the current ns."""
    runtime.register_proc("::foo")
    runtime.register_proc("::bar")

    names = set(runtime.info_commands())
    assert "::foo" in names
    assert "::bar" in names


def test_info_commands_pattern_against_simple_name(runtime: RuntimeHandle):
    """Unqualified patterns glob-match against the simple name,
    but the returned entries are still FQNs."""
    runtime.register_proc("::helper_one")
    runtime.register_proc("::helper_two")
    runtime.register_proc("::unrelated")

    names = set(runtime.info_commands("helper_*"))
    assert names == {"::helper_one", "::helper_two"}


def test_info_commands_qualified_pattern(runtime: RuntimeHandle):
    """Qualified patterns like ``::ns::*`` walk the target ns only."""
    runtime.register_proc("::ns::alpha")
    runtime.register_proc("::ns::beta")
    runtime.register_proc("::other")

    names = set(runtime.info_commands("::ns::*"))
    assert names == {"::ns::alpha", "::ns::beta"}


def test_info_commands_excludes_hidden(runtime: RuntimeHandle):
    """Hidden-table entries are not visible to ``info commands``."""
    root = runtime.root()
    runtime.register_proc("::showing")
    runtime.register_proc("::about_to_hide")

    # Sanity: both show up.
    names = set(runtime.info_commands())
    assert "::showing" in names
    assert "::about_to_hide" in names

    assert runtime.hide(root, "about_to_hide", "about_to_hide") == 0

    names = set(runtime.info_commands())
    assert "::showing" in names
    assert "::about_to_hide" not in names


def test_info_procs_excludes_compiled(runtime: RuntimeHandle):
    """``info procs`` filters out compiled procs (``func_idx !=
    0``) — it should only return interpreted bodies."""
    runtime.register_proc("::interp_proc")
    runtime.register_compiled("::compiled_proc", func_idx=5)

    commands = set(runtime.info_commands())
    assert "::interp_proc" in commands
    assert "::compiled_proc" in commands

    procs = set(runtime.info_procs())
    assert "::interp_proc" in procs
    assert "::compiled_proc" not in procs


def test_info_procs_pattern(runtime: RuntimeHandle):
    """Patterns apply to ``info procs`` the same way."""
    runtime.register_proc("::alpha")
    runtime.register_proc("::beta")
    runtime.register_compiled("::also_compiled")

    procs = set(runtime.info_procs("a*"))
    assert procs == {"::alpha"}


def test_namespace_which_unqualified_resolves_to_fqn(runtime: RuntimeHandle):
    """``namespace which foo`` returns the FQN of the resolved
    Command — ``::foo`` for a root-level proc."""
    runtime.register_proc("::foo")
    assert runtime.namespace_which("foo") == "::foo"


def test_namespace_which_qualified_match(runtime: RuntimeHandle):
    """``namespace which ::ns::foo`` returns ``::ns::foo`` when
    the command lives there."""
    runtime.register_proc("::ns::foo")
    assert runtime.namespace_which("::ns::foo") == "::ns::foo"


def test_namespace_which_miss_returns_empty(runtime: RuntimeHandle):
    """Missing commands return the empty string — probe, not
    error."""
    assert runtime.namespace_which("nonexistent") == ""


def test_namespace_which_survives_hide(runtime: RuntimeHandle):
    """Hiding makes ``namespace which`` return empty for the name
    — hidden commands aren't visible to the resolver."""
    root = runtime.root()
    runtime.register_proc("::vanish")
    assert runtime.namespace_which("vanish") == "::vanish"
    assert runtime.hide(root, "vanish", "vanish") == 0
    assert runtime.namespace_which("vanish") == ""


def test_info_default_missing_arg_raises_error(runtime: RuntimeHandle):
    """``info default`` on an arg not in the proc's params list
    raises ``procedure "X" doesn't have an argument "Y"`` — Tcl 9
    ``InfoDefaultCmd`` wording.  Exercises the interpreted-proc
    path (``proc_register``, not ``proc_register_compiled``) so the
    resolver sees a params TclObj and we reach the inner error."""
    exports = runtime.instance.exports(runtime.store)
    info_default = exports["info_default"]
    catch_enter = exports["catch_enter"]
    catch_leave = exports["catch_leave"]
    catch_has_error = exports["catch_has_error"]
    catch_result = exports["catch_result"]

    # Register an interpreted proc with params ``a b`` via
    # ``proc_register`` (same path as the runtime tests use to
    # stage interpreted bodies).
    runtime.register_proc("::p1", params="a b", body=" ")

    # Stage inputs + a scratch var.
    pn_ptr, pn_len = runtime.stage("::p1")
    proc_obj = runtime.obj_new_string(runtime.store, pn_ptr, pn_len)
    arg_ptr, arg_len = runtime.stage("missing_arg")
    arg_obj = runtime.obj_new_string(runtime.store, arg_ptr, arg_len)
    var_ptr, var_len = runtime.stage("result_var")
    var_obj = runtime.obj_new_string(runtime.store, var_ptr, var_len)

    catch_enter(runtime.store)
    info_default(runtime.store, proc_obj, arg_obj, var_obj)
    has_err = catch_has_error(runtime.store)
    # ``catch_leave`` snapshots ``error_flag`` into
    # ``last_catch_had_error`` and then clears it — ``catch_result``
    # consults the snapshot, so the read has to come after leave.
    catch_leave(runtime.store)
    result_obj = catch_result(runtime.store)

    assert has_err == 1, "info default with missing arg should raise"
    msg = runtime.read_obj_string(result_obj)
    assert msg == 'procedure "::p1" doesn\'t have an argument "missing_arg"'

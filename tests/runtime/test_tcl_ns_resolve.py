"""Unit tests for ``tcl_ns.ns_resolve_qualified`` (P1.4).

Loads only the Zig WASM runtime — no Tcl compilation involved.
Stages name bytes via the test-only ``tcl_test_alloc`` export and
calls the resolver directly so we can assert behaviour for shapes
that wouldn't otherwise show up until P2 wires ``proc_lookup``
through the new tree.

See ``docs/design/runtime/namespace-tree.md`` §5.1 for the
algorithm + §8 for the edge-case behaviour these tests pin.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.runtime._host_imports import define_host_spawn  # noqa: E402


from core.runtime_wasm import runtime_wasm_path  # noqa: E402

_ZIG_RUNTIME_PATH = runtime_wasm_path()


@pytest.fixture()
def runtime():
    """Fresh runtime instance per test — root_addr is module-level
    state in tcl_ns.zig and we want each test to see its own tree.
    """
    if not _ZIG_RUNTIME_PATH.exists():
        pytest.skip(f"Zig WASM runtime not built: {_ZIG_RUNTIME_PATH}")

    engine = wasmtime.Engine()
    store = wasmtime.Store(engine)
    store.set_wasi(wasmtime.WasiConfig())
    linker = wasmtime.Linker(engine)
    linker.define_wasi()

    # The runtime imports ``env.call_compiled_proc`` even though
    # this test never invokes any compiled proc.  Provide a stub
    # that traps if called — it shouldn't be reachable from the
    # ns_resolve_qualified path.
    def _unused_call(name_ptr: int, name_len: int, argv: int, argc: int) -> int:
        raise RuntimeError("call_compiled_proc: unexpected call from ns-resolve test")

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
    """Thin wrapper over a wasmtime instance + store for ergonomic
    access to runtime exports + linear memory."""

    def __init__(self, store: wasmtime.Store, instance: wasmtime.Instance):
        self.store = store
        self.instance = instance
        exports = instance.exports(store)
        self.memory: wasmtime.Memory = exports["memory"]
        self.test_alloc = exports["tcl_test_alloc"]
        self.ns_root = exports["tcl_ns_root"]
        self.ns_lookup = exports["tcl_ns_lookup"]
        self.ns_create = exports["tcl_ns_create"]
        self.ns_resolve = exports["tcl_ns_resolve_qualified"]
        self.last_simple_ptr = exports["tcl_ns_last_simple_ptr"]
        self.last_simple_len = exports["tcl_ns_last_simple_len"]
        self.last_alt = exports["tcl_ns_last_alt"]

    def stage(self, s: str) -> tuple[int, int]:
        """Copy ``s`` into linear memory and return ``(ptr, len)``."""
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

    def lookup(self, parent: int, name: str) -> int:
        ptr, length = self.stage(name)
        return self.ns_lookup(self.store, parent, ptr, length)

    def resolve(self, cxt: int, name: str) -> tuple[int, bytes, int]:
        """Run ``ns_resolve_qualified`` and return
        ``(target_ns, simple_name_bytes, alt_ns)``."""
        ptr, length = self.stage(name)
        target = self.ns_resolve(self.store, cxt, ptr, length)
        sptr = self.last_simple_ptr(self.store)
        slen = self.last_simple_len(self.store)
        simple = self.read(sptr, slen)
        alt = self.last_alt(self.store)
        return target, simple, alt


# -- Tests -------------------------------------------------------------------


def test_root_is_idempotent(runtime: RuntimeHandle):
    a = runtime.root()
    b = runtime.root()
    assert a == b
    assert a != 0


def test_resolve_root_alone(runtime: RuntimeHandle):
    """``::`` (or empty) resolves to root with empty simple name and
    alt_ns == 0 (matches C's ``::``-only special case)."""
    root = runtime.root()
    target, simple, alt = runtime.resolve(0, "::")
    assert target == root
    assert simple == b""
    assert alt == 0

    target, simple, alt = runtime.resolve(root, "")
    assert target == root
    assert simple == b""


def test_resolve_simple_unqualified_from_root(runtime: RuntimeHandle):
    """Unqualified ``foo`` from root: target_ns=root, simple='foo',
    alt=0 (no alt because we're already at root)."""
    root = runtime.root()
    target, simple, alt = runtime.resolve(root, "foo")
    assert target == root
    assert simple == b"foo"
    assert alt == 0


def test_resolve_fully_qualified_existing(runtime: RuntimeHandle):
    """``::a::b::cmd`` → walks root.a.b, returns (b_ns, 'cmd', 0).
    Anchored at root, so no alt path."""
    root = runtime.root()
    a = runtime.create(root, "a")
    b = runtime.create(a, "b")

    target, simple, alt = runtime.resolve(0, "::a::b::cmd")
    assert target == b
    assert simple == b"cmd"
    assert alt == 0


def test_resolve_fully_qualified_missing_intermediate(runtime: RuntimeHandle):
    """``::nope::cmd`` with no ``::nope`` ns → target_ns=0 but the
    walk continues so simple_name still reports the trailing ``cmd``
    (matches C's ``Tcl_FindHashEntry`` returning NULL while the
    parsing loop completes)."""
    target, simple, alt = runtime.resolve(0, "::nope::cmd")
    assert target == 0
    assert simple == b"cmd"
    assert alt == 0


def test_resolve_partial_qualification_uses_alt_path(runtime: RuntimeHandle):
    """From context ``::ctx`` looking up ``a::cmd``: primary walks
    ctx.a, alt walks root.a.  Both can resolve independently."""
    root = runtime.root()
    ctx = runtime.create(root, "ctx")
    ctx_a = runtime.create(ctx, "a")
    root_a = runtime.create(root, "a")

    target, simple, alt = runtime.resolve(ctx, "a::cmd")
    assert target == ctx_a
    assert simple == b"cmd"
    assert alt == root_a


def test_resolve_partial_qualification_alt_only(runtime: RuntimeHandle):
    """From ``::ctx`` looking up ``a::cmd`` when only ``::a`` exists
    (no ``::ctx::a``): primary side flips to 0 mid-walk, alt walks
    root.a successfully."""
    root = runtime.root()
    ctx = runtime.create(root, "ctx")
    root_a = runtime.create(root, "a")

    target, simple, alt = runtime.resolve(ctx, "a::cmd")
    assert target == 0  # ctx.a doesn't exist
    assert simple == b"cmd"
    assert alt == root_a


def test_resolve_alt_disabled_when_anchored_at_root(runtime: RuntimeHandle):
    """When primary path is already root (``::a::b`` from any cxt),
    alt_ns stays 0 — there's no point searching root twice."""
    root = runtime.root()
    a = runtime.create(root, "a")
    runtime.create(a, "b")

    target, simple, alt = runtime.resolve(root, "::a::b::cmd")
    assert target != 0  # ``a.b`` exists
    assert simple == b"cmd"
    assert alt == 0


def test_resolve_repeated_colons_treated_as_single_separator(
    runtime: RuntimeHandle,
):
    """``:::a::::b`` is the same as ``::a::b`` — Tcl semantics for
    runs of two-or-more adjacent colons."""
    root = runtime.root()
    a = runtime.create(root, "a")
    b = runtime.create(a, "b")

    target, simple, alt = runtime.resolve(0, ":::a::::b::cmd")
    assert target == b
    assert simple == b"cmd"
    assert alt == 0


def test_resolve_trailing_colons_returns_namespace_itself(
    runtime: RuntimeHandle,
):
    """``::a::`` resolves to ``::a`` itself with an empty simple
    name (no trailing component)."""
    root = runtime.root()
    a = runtime.create(root, "a")

    target, simple, alt = runtime.resolve(0, "::a::")
    assert target == a
    assert simple == b""
    assert alt == 0


def test_resolve_ns_create_is_find_or_create(runtime: RuntimeHandle):
    """Calling ``ns_create`` for an existing child returns the same
    handle — idempotent."""
    root = runtime.root()
    a1 = runtime.create(root, "a")
    a2 = runtime.create(root, "a")
    assert a1 == a2


def test_resolve_lookup_returns_zero_for_missing(runtime: RuntimeHandle):
    root = runtime.root()
    assert runtime.lookup(root, "nope") == 0

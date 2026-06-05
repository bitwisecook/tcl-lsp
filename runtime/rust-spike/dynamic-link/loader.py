# ============================================================================
# SPIKE -- throwaway proof-of-concept. NOT the final design.
# Do not derive the production loader/ABI shape from this file; see
# runtime/rust-spike/README.md and docs/design/runtime/c-extension-abi.md.
# ============================================================================
#
# Dynamic-linking C-extension spike: the host loader.
#
# Instantiates the Rust runtime module, then loads the PIC *side module*
# (pkga.c built `-fPIC -shared`) the way a real `package require` would:
#   - parse its `dylink.0` section for memory/table size needs,
#   - reserve a memory region (-> __memory_base) and table slots (-> __table_base)
#     from the *runtime's* shared memory and shared function table,
#   - instantiate the side module against those bases + the runtime's Tcl C API,
#   - run __wasm_apply_data_relocs + __wasm_call_ctors, then Pkga_Init,
#   - and finally dispatch the registered commands through the runtime, which
#     `call_indirect`s back into the side module via the shared table.
#
# Usage: python loader.py <runtime.wasm> <pkga.side.wasm>

import struct
import sys

from wasmtime import (
    Engine,
    Global,
    GlobalType,
    Instance,
    Module,
    Store,
    Val,
    ValType,
)


def _leb(b, o):
    r = s = 0
    while True:
        x = b[o]
        o += 1
        r |= (x & 0x7F) << s
        if not x & 0x80:
            return r, o
        s += 7


def parse_dylink_meminfo(wasm_bytes):
    """Return (mem_size, mem_align, table_size, table_align) from dylink.0."""
    o = 8
    while o < len(wasm_bytes):
        sid = wasm_bytes[o]
        o += 1
        size, o = _leb(wasm_bytes, o)
        body = wasm_bytes[o : o + size]
        o += size
        if sid != 0:
            continue
        n, p = _leb(body, 0)
        if body[p : p + n] != b"dylink.0":
            continue
        p += n
        # First subsection is WASM_DYLINK_MEM_INFO (id 1).
        _subid = body[p]
        p += 1
        _subsize, p = _leb(body, p)
        msize, p = _leb(body, p)
        malign, p = _leb(body, p)
        tsize, p = _leb(body, p)
        talign, p = _leb(body, p)
        return msize, malign, tsize, talign
    raise SystemExit("side module has no dylink.0 section")


def main():
    runtime_path, side_path = sys.argv[1], sys.argv[2]
    side_bytes = open(side_path, "rb").read()

    engine = Engine()
    store = Store(engine)

    # 1. Instantiate the Rust runtime (freestanding wasm32-unknown-unknown: no
    #    imports of its own).
    rt_module = Module(engine, open(runtime_path, "rb").read())
    rt = Instance(store, rt_module, [])
    rx = rt.exports(store)
    mem = rx["memory"]
    table = rx["__indirect_function_table"]

    def call(name, *args):
        return rx[name](store, *args)

    # 2. Parse the side module's footprint and reserve memory + table from the
    #    runtime (this is the loader's job that the runtime language never sees).
    msize, _malign, tsize, _talign = parse_dylink_meminfo(side_bytes)
    memory_base = call("spike_alloc", msize if msize else 1)
    table_base = table.size(store)
    table.grow(store, tsize, None)  # null-funcref fill; element seg overwrites

    # Dedicated C stack for the side module (grows down), disjoint from the
    # runtime's own stack.
    STACK = 64 * 1024
    stack_lo = call("spike_alloc", STACK)
    stack_hi = (stack_lo + STACK) & ~15

    def g(value, mutable):
        return Global(store, GlobalType(ValType.i32(), mutable), Val.i32(value))

    provided = {
        "memory": mem,
        "__indirect_function_table": table,
        "__memory_base": g(memory_base, False),
        "__table_base": g(table_base, False),
        "__stack_pointer": g(stack_hi, True),
    }

    # 3. Build the side module's import list in declaration order, resolving Tcl_*
    #    to the runtime's exports -- the extension needs no per-extension shim.
    imports = []
    for imp in side_module_imports(side_bytes):
        if imp in provided:
            imports.append(provided[imp])
        else:
            fn = rx.get(imp)
            if fn is None:
                raise SystemExit(f"unsatisfied side-module import: env.{imp}")
            imports.append(fn)

    side_module = Module(engine, side_bytes)
    side = Instance(store, side_module, imports)
    sx = side.exports(store)

    # 4. Relocate data, run ctors, then the (unmodified) extension's init.
    if "__wasm_apply_data_relocs" in [e.name for e in side_module.exports]:
        sx["__wasm_apply_data_relocs"](store)
    if "__wasm_call_ctors" in [e.name for e in side_module.exports]:
        sx["__wasm_call_ctors"](store)

    interp = call("spike_new_interp")
    init_rc = sx["Pkga_Init"](store, interp)

    print("== Rust-runtime C-extension spike (DYNAMIC side-module loading) ==")
    print(f"side module: {side_path}")
    print(f"  loaded at  __memory_base={memory_base} __table_base={table_base} "
          f"(needs {msize}B data, {tsize} table slots)")
    print(f"Pkga_Init(interp) -> {init_rc} (TCL_OK = {init_rc == 0})")
    print()

    def eval_cmd(words):
        objs = []
        for w in words:
            wb = w.encode("utf-8")
            p = call("spike_alloc", len(wb))
            mem.write(store, wb, p)
            objs.append(call("spike_make_string_obj", p, len(wb)))
        argv = call("spike_alloc", 4 * len(objs))
        mem.write(store, b"".join(struct.pack("<i", o) for o in objs), argv)
        rc = call("spike_dispatch", interp, len(objs), argv)
        rp = call("spike_result_ptr", interp)
        rl = call("spike_result_len", interp)
        out = bytes(mem.read(store, rp, rp + rl)).decode("utf-8") if rp else ""
        return rc, out

    ok = init_rc == 0
    cases = [
        (["pkga_eq", "hello", "hello"], "1"),
        (["pkga_eq", "hello", "world"], "0"),
        (["pkga_eq", "café", "café"], "1"),  # multi-byte UTF-8
        (["pkga_quote", "round-trip-value"], "round-trip-value"),
    ]
    for words, want in cases:
        rc, got = eval_cmd(words)
        passed = rc == 0 and got == want
        ok = ok and passed
        print(f"  {' '.join(words):<34} -> {got!r:<18} [{'PASS' if passed else 'FAIL'}]")

    print()
    print("SPIKE PASS" if ok else "SPIKE FAIL")
    sys.exit(0 if ok else 1)


def side_module_imports(wasm_bytes):
    """Import field names (env.*) in declaration order, via a tiny parser so we
    do not depend on the exact wasmtime ImportType attribute surface."""
    o = 8
    names = []
    while o < len(wasm_bytes):
        sid = wasm_bytes[o]
        o += 1
        size, o = _leb(wasm_bytes, o)
        body = wasm_bytes[o : o + size]
        o += size
        if sid != 2:
            continue
        p = 0
        cnt, p = _leb(body, p)
        for _ in range(cnt):
            ml, p = _leb(body, p)
            p += ml  # module name (always "env" / "GOT.*")
            nl, p = _leb(body, p)
            nm = body[p : p + nl].decode()
            p += nl
            k = body[p]
            p += 1
            if k == 0:  # func: typeidx
                _, p = _leb(body, p)
            elif k == 1:  # table: reftype + limits
                p += 1
                fl = body[p]
                p += 1
                _, p = _leb(body, p)
                if fl:
                    _, p = _leb(body, p)
            elif k == 2:  # mem: limits
                fl = body[p]
                p += 1
                _, p = _leb(body, p)
                if fl:
                    _, p = _leb(body, p)
            elif k == 3:  # global: valtype + mutability
                p += 2
            names.append(nm)
    return names


if __name__ == "__main__":
    main()

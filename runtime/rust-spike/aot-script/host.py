#!/usr/bin/env python3
"""AOT-script host: link an emitted Tcl `::top` module against the real Rust
runtime (sharing the runtime's linear memory), bootstrap an interp, run `::top`,
and read back the resulting variables via the runtime's `Tcl_GetString`.

Run: uv run --with wasmtime python host.py <runtime.wasm> <user.wasm>

The emitted module must be built with the constant pool relocated to the
runtime's reserved gap (`emit_wasm` passes `RESERVED_DATA_BASE`) — at base 0 the
first boxed command lands at offset 0 and `tcl_obj_new_string(ptr=0, …)` reads as
a null/empty pointer, silently dropping that command.
"""

import sys

from wasmtime import Engine, Linker, Module, Store, WasiConfig

rt_path, user_path = sys.argv[1], sys.argv[2]
engine = Engine()
store = Store(engine)

wasi = WasiConfig()
wasi.inherit_stdout()
wasi.inherit_stderr()
store.set_wasi(wasi)

linker = Linker(engine)
linker.define_wasi()

rt = linker.instantiate(store, Module(engine, open(rt_path, "rb").read()))
rx = rt.exports(store)
mem = rx["memory"]

# Bootstrap an interp and make it current (the codegen ABI evaluates against it).
interp = rx["tcl_runtime_create_interp"](store)
rx["tcl_runtime_set_current_interp"](store, interp)


def box(s: bytes) -> int:
    off = mem.grow(store, 1) * 65536  # guaranteed-free page
    mem.write(store, s, off)
    return rx["tcl_obj_new_string"](store, off, len(s))


def getstr(obj: int) -> bytes:
    p = rx["Tcl_GetString"](store, obj)
    return bytes(mem.read(store, p, p + 256)).split(b"\x00", 1)[0]


def evals(script: bytes) -> bytes:
    return getstr(rx["tcl_eval"](store, box(script)))


# Link the emitted module: its "tcl" imports resolve to the runtime's exports,
# and it shares the runtime's linear memory.
ulinker = Linker(engine)
for fn in ("tcl_eval", "tcl_obj_new_string", "tcl_expr_bool", "tcl_obj_release"):
    ulinker.define(store, "tcl", fn, rx[fn])
ulinker.define(store, "tcl", "memory", mem)

user = ulinker.instantiate(store, Module(engine, open(user_path, "rb").read()))

print("running ::top ...")
user.exports(store)["::top"](store)
print("::top returned")

# Read back a few common probe variables (ignore those the script didn't set).
for var in (b"x", b"n", b"doubled", b"greeting"):
    val = evals(b"set " + var)
    if val:
        print(f"  {var.decode()} = {val!r}")

#!/usr/bin/env python3
"""AOT-script host: link an emitted Tcl `::top` module against the real Rust
runtime (sharing the runtime's linear memory), bootstrap an interp, run `::top`,
and verify the side effect via the runtime's `tcl_expr_bool`.

Run: uv run --with wasmtime python aot_host.py <runtime.wasm> <user.wasm>
"""
import sys
from wasmtime import Engine, Store, Module, Linker, WasiConfig

rt_path, user_path = sys.argv[1], sys.argv[2]
engine = Engine()
store = Store(engine)

wasi = WasiConfig()
wasi.inherit_stdout()
wasi.inherit_stderr()
store.set_wasi(wasi)

linker = Linker(engine)
linker.define_wasi()

rt_mod = Module(engine, open(rt_path, "rb").read())
rt = linker.instantiate(store, rt_mod)
rx = rt.exports(store)

# wasip1 reactor: run _initialize (ctors) before any other export.
if "_initialize" in [e.name for e in rt_mod.exports]:
    rx["_initialize"](store)

# Bootstrap an interp and make it current (the codegen ABI evaluates against it).
interp = rx["tcl_runtime_create_interp"](store)
rx["tcl_runtime_set_current_interp"](store, interp)
print(f"interp created: {interp}")

mem = rx["memory"]

# Link the emitted module: its "tcl" imports resolve to the runtime's exports,
# and it shares the runtime's linear memory.
ulinker = Linker(engine)
for fn in ("tcl_eval", "tcl_obj_new_string", "tcl_expr_bool", "tcl_obj_release"):
    ulinker.define(store, "tcl", fn, rx[fn])
ulinker.define(store, "tcl", "memory", mem)

user_mod = Module(engine, open(user_path, "rb").read())
user = ulinker.instantiate(store, user_mod)
ux = user.exports(store)

def _box(s: bytes) -> int:
    old_pages = mem.grow(store, 1)
    off = old_pages * 65536
    mem.write(store, s, off)
    return rx["tcl_obj_new_string"](store, off, len(s))

def evals(script: bytes):
    rx["tcl_obj_release"](store, rx["tcl_eval"](store, _box(script)))

def expr_bool(expr: bytes) -> int:
    return rx["tcl_expr_bool"](store, _box(expr))

# (a0) Does the interp evaluate variable-free expressions at all?
print(f"interp sanity (1 < 2):       {expr_bool(b'1 < 2')}")
print(f"interp sanity (2 + 2 == 4):  {expr_bool(b'2 + 2 == 4')}")
# (a) Host-driven eval + variable persistence?
evals(b"set y 7")
print(f"host eval sanity ($y == 7): {expr_bool(b'$y == 7')}")

# (b) Run the AOT-compiled script.
print("running ::top ...")
ux["::top"](store)
print("::top returned without trapping")

# (c) Did ::top's commands land?
print(f"after ::top  [info exists x]: {expr_bool(b'[info exists x]')}")
print(f"after ::top  ($x == 42):      {expr_bool(b'$x == 42')}")
print(f"after ::top  ($x == 41):      {expr_bool(b'$x == 41')}")
ok = expr_bool(b"$x == 42")
sys.exit(0 if ok == 1 else 1)

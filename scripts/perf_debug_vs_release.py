#!/usr/bin/env python3
"""Compare Debug vs ReleaseFast Zig runtime on a fixed workload."""

from __future__ import annotations

import json
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

import wasmtime  # noqa: E402

from core.compiler.cfg import build_cfg  # noqa: E402
from core.compiler.codegen.wasm import wasm_codegen_module  # noqa: E402
from core.compiler.lowering import lower_to_ir  # noqa: E402
from core.runtime_wasm import runtime_wasm_path  # noqa: E402

OUTPUT = REPO / "tmp" / "perf-output"
RELEASE_RT = runtime_wasm_path()
DEBUG_RT = OUTPUT / "tcl_runtime_debug.wasm"

WORKLOADS = {
    "set+incr (50k)": ("set x 0\nfor {set i 0} {$i < 50000} {incr i} { incr x }\n"),
    "if/else (20k)": (
        "set t 0\nfor {set i 0} {$i < 20000} {incr i} { "
        "if {$i > 0} { incr t } else { incr t -1 } }\n"
    ),
    "proc call no-args (20k)": (
        "proc f {} { return 42 }\nfor {set i 0} {$i < 20000} {incr i} { f }\n"
    ),
}


def _compile(src: str, fname: str) -> bytes:
    ir = lower_to_ir(src)
    cfg = build_cfg(ir)
    return wasm_codegen_module(cfg, ir, optimise=False).to_bytes()


def _run_with_runtime(rt_path: Path, wasm: bytes) -> float:
    """Run ``wasm`` with the Zig runtime at ``rt_path`` and return
    the wall-time (ns) of one invocation of ``::top``."""
    engine = wasmtime.Engine()
    rt_module = wasmtime.Module.from_file(engine, str(rt_path))

    store = wasmtime.Store(engine)
    wasi = wasmtime.WasiConfig()
    with tempfile.TemporaryDirectory() as preopen:
        wasi.preopen_dir(preopen, "/")
        store.set_wasi(wasi)

        linker = wasmtime.Linker(engine)
        linker.define_wasi()

        # call_compiled_proc that just raises (we don't exercise
        # cross-context here).
        def _ccp(name_ptr, name_len, argv_ptr, argc):
            raise RuntimeError("no compiled-proc dispatch in this bench")

        ftype = wasmtime.FuncType([wasmtime.ValType.i32()] * 4, [wasmtime.ValType.i32()])
        linker.define(
            store,
            "env",
            "call_compiled_proc",
            wasmtime.Func(store, ftype, _ccp),
        )

        rt_inst = linker.instantiate(store, rt_module)
        init_fn = rt_inst.exports(store).get("_initialize")
        if init_fn is not None:
            init_fn(store)
        for name in (e.name for e in rt_module.exports):
            if name.startswith("__"):
                continue
            val = rt_inst.exports(store)[name]
            if isinstance(val, (wasmtime.Func, wasmtime.Memory)):
                linker.define(store, "tcl", name, val)

        tcl_module = wasmtime.Module(engine, wasm)
        tcl_inst = linker.instantiate(store, tcl_module)
        top = tcl_inst.exports(store)["::top"]
        t0 = time.perf_counter_ns()
        top(store)
        return time.perf_counter_ns() - t0


def main():
    out = []
    for label, src in WORKLOADS.items():
        wasm = _compile(src, label)
        for name, rt in (("release", RELEASE_RT), ("debug", DEBUG_RT)):
            samples = []
            for _ in range(3):  # warmup not measured
                _run_with_runtime(rt, wasm)
            for _ in range(7):
                samples.append(_run_with_runtime(rt, wasm))
            samples.sort()
            med = samples[len(samples) // 2]
            out.append(
                {
                    "workload": label,
                    "build": name,
                    "median_ns": med,
                    "all_ns": samples,
                    "runtime_bytes": rt.stat().st_size,
                }
            )
            print(
                f"{label:30s}  {name:8s}  median={med / 1e6:7.2f} ms  "
                f"runtime={rt.stat().st_size / 1024 / 1024:.1f} MB",
                file=sys.stderr,
            )
    (OUTPUT / "debug_vs_release.json").write_text(json.dumps(out, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

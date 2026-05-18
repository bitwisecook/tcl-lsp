#!/usr/bin/env python3
"""Run the in-scope tcltest suite under the leak-check runtime build
and emit per-file (alloc, double_free) residual counts.

S0.4 deliverable.  Usage:

    make build-runtime-leakcheck
    uv run python scripts/leak_sweep.py [--no-frame-elision] [--out PATH]
    uv run python scripts/diff_leak_sweep.py    # diff vs baseline

The leakcheck binary is the same shape as the production runtime
plus four extra exports (``tcl_test_alloc_count``,
``tcl_test_double_free_count``, ``tcl_test_reset_counters``,
``tcl_test_finalize``).  The script:

1. resets the counters,
2. runs the bundled test through ``tcl_eval``,
3. calls ``tcl_test_finalize`` (which drains the deferred-free
   queue), and
4. records the residual counts.

Output: ``tmp/perf-output/leak_sweep_results.json`` with one row per
in-scope test:

    {
        "stem": "parse",
        "subsystem": "parsing",
        "alloc_residual": 1234,
        "double_free": 0,
        "retain_after_defer": 0,
        "trapped": false,
        "trap_message": null
    }
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import time
import traceback
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))

# Force the runtime path before importing _run_wasm so the
# leak-check binary is loaded.  The default path is the same place
# ``make build-runtime-leakcheck`` writes to, so this is a no-op in
# the common workflow; the env var lets a contributor point at a
# saved binary for offline analysis.
from core.runtime_wasm import DEFAULT_PATH as LEAKCHECK_WASM  # noqa: E402

os.environ.setdefault("TCL_LSP_RUNTIME_WASM", str(LEAKCHECK_WASM))

import wasmtime  # noqa: E402

from tests.external.run_tcl9_tests import _IN_SCOPE, _bundle  # noqa: E402
from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl,
    _define_call_compiled_proc,
    _get_engine_with_timeout,
)

OUTPUT = REPO / "tmp" / "perf-output" / "leak_sweep_results.json"
TIMEOUT_S = 15.0


def _verify_leakcheck_binary() -> None:
    """Sanity-check that the binary actually has the leak-check
    exports.  A production build will not, and silently producing
    all-zero counts would be confusing.
    """
    if not LEAKCHECK_WASM.exists():
        raise SystemExit(
            f"runtime not built: {LEAKCHECK_WASM}\nrun 'make build-runtime-leakcheck' first."
        )
    engine = wasmtime.Engine()
    module = wasmtime.Module.from_file(engine, str(LEAKCHECK_WASM))
    export_names = {imp.name for imp in module.exports}
    required = {
        "tcl_test_alloc_count",
        "tcl_test_double_free_count",
        "tcl_test_retain_after_defer_count",
        "tcl_test_reset_counters",
        "tcl_test_finalize",
    }
    missing = required - export_names
    if missing:
        raise SystemExit(
            f"binary at {LEAKCHECK_WASM} is missing leak-check exports "
            f"{missing}.  rebuild with -Dleak-check=true."
        )


def _run_one_with_counters(
    bundle_src: str,
    label: str,
    source_dir: Path,
    *,
    frame_elision: bool = True,
) -> dict[str, Any]:
    """Compile and run *bundle_src* against the leakcheck runtime,
    returning the residual counter readings.

    Reuses the same wasmtime engine across calls (per-call ``Store``
    isolates state).  Matches the structure of ``_run_wasm`` in
    ``tests/test_wasm_real_tcl.py`` but exposes the four counter
    exports the leak-check binary provides.
    """
    out: dict[str, Any] = {
        "label": label,
        "trapped": False,
        "trap_message": None,
    }
    try:
        wasm_bytes = _compile_tcl(
            bundle_src,
            source_dir=source_dir,
            frame_elision=frame_elision,
        )
    except BaseException as exc:
        out["compile_error"] = str(exc)[-400:]
        return out
    out["wasm_bytes"] = len(wasm_bytes)

    engine = _get_engine_with_timeout()
    store = wasmtime.Store(engine)

    # Park the deadline far in the future for the setup work
    # (instantiate, _initialize, counter resets).  We rearm a tight
    # deadline just before invoking the user proc.
    from tests.test_wasm_real_tcl import _epoch_count, _epoch_count_set

    store.set_epoch_deadline(_epoch_count + 1_000_000)

    wasi_config = wasmtime.WasiConfig()
    with tempfile.TemporaryDirectory(prefix="tcl9leak-") as host_tmp:
        wasi_config.preopen_dir(host_tmp, "/")
        store.set_wasi(wasi_config)

        rt_module = wasmtime.Module.from_file(engine, str(LEAKCHECK_WASM))
        linker = wasmtime.Linker(engine)
        linker.define_wasi()
        tcl_box, mem_box, rt_box = _define_call_compiled_proc(linker, store)
        rt_inst = linker.instantiate(store, rt_module)
        rt_box[0] = rt_inst
        init = rt_inst.exports(store).get("_initialize")
        if init is not None:
            init(store)

        # Bridge runtime exports under the ``tcl`` import module so
        # the user wasm sees ``tcl::tcl_cmd_*`` etc.  Mirrors
        # ``_run_wasm`` in ``tests/test_wasm_real_tcl.py``.
        for export in rt_module.exports:
            name = export.name
            if name.startswith("__"):
                continue
            val = rt_inst.exports(store)[name]
            if isinstance(val, wasmtime.Func):
                linker.define(store, "tcl", name, val)
            elif name == "memory":
                linker.define(store, "tcl", name, val)

        reset = rt_inst.exports(store)["tcl_test_reset_counters"]
        alloc_count = rt_inst.exports(store)["tcl_test_alloc_count"]
        double_free = rt_inst.exports(store)["tcl_test_double_free_count"]
        retain_after_defer = rt_inst.exports(store)["tcl_test_retain_after_defer_count"]
        finalize = rt_inst.exports(store)["tcl_test_finalize"]
        _ = alloc_count  # readable mid-run if needed

        reset(store)

        user_module = wasmtime.Module(engine, wasm_bytes)
        try:
            user_inst = linker.instantiate(store, user_module)
        except BaseException as exc:
            out["instantiate_error"] = str(exc)[-200:]
            return out
        tcl_box[0] = user_inst
        mem_box[0] = rt_inst.exports(store)["memory"]
        top = user_inst.exports(store).get("::top")
        if top is None:
            out["instantiate_error"] = "no ::top export"
            return out

        # Guard the user-script run with the engine's epoch deadline
        # so a runaway loop cannot stall the sweep.  Mirrors the
        # pattern in ``tests/test_wasm_real_tcl.py::_run_wasm``: pick
        # an absolute deadline one tick ahead of the current count;
        # the watchdog timer increments the engine to match.
        import threading

        # Rearm the deadline tightly for the user-proc run.
        deadline_value = _epoch_count + 1
        store.set_epoch_deadline(deadline_value)
        watchdog_fired = [False]

        def _bump():
            engine.increment_epoch()
            watchdog_fired[0] = True

        timer = threading.Timer(TIMEOUT_S, _bump)
        timer.daemon = True
        timer.start()
        try:
            top(store)
        except BaseException as exc:
            out["trapped"] = True
            out["trap_message"] = str(exc)[-300:]
        finally:
            timer.cancel()
            if watchdog_fired[0]:
                _epoch_count_set(deadline_value)

        # Even on trap, finalize to drain any queued frees and read
        # the residual.  After a watchdog fire, the engine's epoch
        # is already past the store's deadline; advance the deadline
        # so the finalize / counter calls can run.
        try:
            store.set_epoch_deadline(_epoch_count + 1_000_000)
            out["alloc_residual"] = int(finalize(store))
            out["double_free"] = int(double_free(store))
            out["retain_after_defer"] = int(retain_after_defer(store))
        except BaseException as exc:
            out["counter_read_error"] = str(exc)[-200:]
        if watchdog_fired[0] and not out["trapped"]:
            out["trapped"] = True
            out["trap_message"] = "watchdog interrupt"
    return out


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--no-frame-elision",
        dest="frame_elision",
        action="store_false",
        default=True,
        help="compile every proc with wants_frame=True (S1.1 floor)",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=OUTPUT,
        help=f"output JSON path (default: {OUTPUT.relative_to(REPO)})",
    )
    args = parser.parse_args()

    _verify_leakcheck_binary()
    args.out.parent.mkdir(parents=True, exist_ok=True)

    rows: list[dict[str, Any]] = []
    elision_label = "ON" if args.frame_elision else "OFF"
    print(
        f"in-scope: {len(_IN_SCOPE)} test files  (frame_elision={elision_label})",
        file=sys.stderr,
    )
    for stem, subsystem in _IN_SCOPE:
        test_path = REPO / "tmp" / "tcl9.0.3" / "tests" / f"{stem}.test"
        if not test_path.exists():
            print(f"  [{stem}] missing test file, skipping", file=sys.stderr)
            continue
        try:
            bundle_src = _bundle(test_path)
        except BaseException as exc:
            rows.append(
                {
                    "stem": stem,
                    "subsystem": subsystem,
                    "bundle_error": str(exc)[-200:],
                }
            )
            continue
        t0 = time.perf_counter()
        try:
            row = _run_one_with_counters(
                bundle_src,
                f"tcl9_{stem}.test",
                source_dir=test_path.parent,
                frame_elision=args.frame_elision,
            )
        except BaseException:
            row = {"crashed": traceback.format_exc()[-400:]}
        row.update(stem=stem, subsystem=subsystem, run_s=round(time.perf_counter() - t0, 2))
        rows.append(row)
        print(
            f"  {stem}: residual={row.get('alloc_residual', '?'):>8}  "
            f"double_free={row.get('double_free', '?'):>3}  "
            f"rad={row.get('retain_after_defer', '?'):>6}  "
            f"trapped={row.get('trapped', False)}  "
            f"({row['run_s']:.1f}s)",
            file=sys.stderr,
        )
        # Flush incrementally so a hung file doesn't lose the report.
        args.out.write_text(json.dumps(rows, indent=2, sort_keys=True))

    print(f"\nWrote {args.out.relative_to(REPO)}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

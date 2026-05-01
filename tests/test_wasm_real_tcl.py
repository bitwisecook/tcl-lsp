"""WASM real Tcl tests — compile real .tcl files and verify output.

Tests compile Tcl source files to WASM, link with the Zig runtime,
execute in wasmtime, and verify either:
  - the return value of the top-level script, or
  - the captured stdout (puts output)

This is the "run real Tcl code" validation for the WASM VM.
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path

import pytest

from core.compiler.cfg import build_cfg
from core.compiler.codegen.wasm import DiagMap, wasm_codegen_module
from core.compiler.lowering import lower_to_ir

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from core.runtime_wasm import runtime_wasm_path  # noqa: E402

_ZIG_RUNTIME_PATH = runtime_wasm_path()

_SNIPPETS_DIR = Path(__file__).resolve().parent / "bytecode_snippets"
_FIXTURES_DIR = Path(__file__).resolve().parent / "fixtures"
_EXTERNAL_DIR = Path(__file__).resolve().parent / "external"

_engine: wasmtime.Engine | None = None
_rt_module: wasmtime.Module | None = None
_engine_with_timeout: wasmtime.Engine | None = None
_rt_module_with_timeout: wasmtime.Module | None = None
# Tracks how many times we've called ``increment_epoch`` on the
# timeout engine.  ``set_epoch_deadline`` takes an *absolute* counter
# value: each watchdog fire bumps ``engine.epoch`` permanently, so
# subsequent stores that set deadline=1 trap immediately because the
# engine's counter has already passed that value from previous
# invocations.  Compute the per-call deadline as ``_epoch_count + 1``
# and update ``_epoch_count`` after the watchdog fires (or after the
# call returns clean) so we stay one step ahead.
_epoch_count: int = 0


def _epoch_count_set(v: int) -> None:
    """Update the cached counter; only used on watchdog fire."""
    global _epoch_count
    _epoch_count = max(_epoch_count, v)


def _get_engine() -> wasmtime.Engine:
    global _engine
    if _engine is None:
        _engine = wasmtime.Engine()
    return _engine


def _get_engine_with_timeout() -> wasmtime.Engine:
    """Engine variant with epoch-interruption enabled.

    Used by long-running sweeps (``scripts/run_tcl9_tcltest_sweep.py``)
    that need to bound per-test execution against runaway loops.  A
    watchdog ``threading.Timer`` bumps the engine's epoch after the
    deadline; the next wasm op then traps with an epoch-deadline
    error that propagates to Python as ``wasmtime.Trap``.

    Kept separate from the default engine so the pytest suite (which
    doesn't need timeouts and shouldn't pay the epoch-check overhead)
    keeps using the lean engine.
    """
    global _engine_with_timeout
    if _engine_with_timeout is None:
        cfg = wasmtime.Config()
        cfg.epoch_interruption = True
        _engine_with_timeout = wasmtime.Engine(cfg)
    return _engine_with_timeout


def _get_rt_module() -> wasmtime.Module:
    global _rt_module
    if _rt_module is None:
        if not _ZIG_RUNTIME_PATH.exists():
            pytest.skip(f"Zig WASM runtime not built: {_ZIG_RUNTIME_PATH}")
        _rt_module = wasmtime.Module.from_file(_get_engine(), str(_ZIG_RUNTIME_PATH))
    return _rt_module


def _get_rt_module_with_timeout() -> wasmtime.Module:
    """Re-compile the runtime module against the timeout-enabled engine.

    wasmtime modules are bound to the engine they were compiled
    against, so the timeout variant needs its own pre-compiled copy.
    Cached after the first call.
    """
    global _rt_module_with_timeout
    if _rt_module_with_timeout is None:
        if not _ZIG_RUNTIME_PATH.exists():
            raise RuntimeError(f"Zig WASM runtime not built: {_ZIG_RUNTIME_PATH}")
        _rt_module_with_timeout = wasmtime.Module.from_file(
            _get_engine_with_timeout(),
            str(_ZIG_RUNTIME_PATH),
        )
    return _rt_module_with_timeout


def _compile_tcl(
    source: str,
    *,
    source_dir: Path | None = None,
    frame_elision: bool = True,
) -> bytes:
    """Compile Tcl source to WASM bytes.

    *source_dir* enables compile-time inlining of ``source LITERAL``
    calls — the inliner reads the target off the host filesystem and
    splices its IR into the caller's module so the WASM runs without
    needing a WASI preopen of the test fixtures.  Leave ``None`` to
    disable inlining (compatible with the historic behaviour).
    """
    from core.compiler.source_inliner import inline_static_sources

    ir_module = lower_to_ir(source)
    if source_dir is not None:
        ir_module = inline_static_sources(ir_module, source_dir=source_dir)
    cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(
        cfg_module,
        ir_module,
        optimise=False,
        frame_elision=frame_elision,
    )
    return wasm_module.to_bytes()


def _compile_tcl_with_diag(
    source: str,
    filename: str = "<inline>",
    *,
    source_dir: Path | None = None,
) -> tuple[bytes, DiagMap]:
    """Compile Tcl source and return the bytes + populated diag map.

    The map resolves ``site=<id>`` prefixes the runtime writes on trap
    back to ``(file, line, col, command)``.  Used by the external
    runner to decode trap output inline.

    *source_dir* enables compile-time ``source`` inlining — see
    :func:`_compile_tcl` for details.  When omitted the runtime
    handles ``source`` calls via the WASI preopen as before.
    """
    from core.compiler.source_inliner import inline_static_sources

    ir_module = lower_to_ir(source)
    if source_dir is not None:
        ir_module = inline_static_sources(ir_module, source_dir=source_dir)
    cfg_module = build_cfg(ir_module)
    diag_map = DiagMap(filename=filename)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=False, diag_map=diag_map)
    return wasm_module.to_bytes(), diag_map


def _compile_tcl_as_proc(source: str) -> tuple[bytes, str]:
    """Wrap source in a __main__ proc so we get a return value.

    Returns (wasm_bytes, proc_name).
    """
    wrapped = f"proc __main__ {{}} {{\n{source}\n}}\n"
    return _compile_tcl(wrapped), "::__main__"


def _run_wasm(
    wasm_bytes: bytes,
    capture_stdout: bool = False,
    func_name: str = "::top",
    args: tuple[int, ...] = (),
    capture_stderr: bool = False,
    preopen_tmpdir: str | None = None,
    timeout_s: float | None = None,
    memory_size: int | None = None,
    capabilities: int = 0,
    host_spawn=None,
) -> tuple:
    """Link and run a compiled Tcl WASM module.

    Returns ``(return_value, stdout_text)`` by default.  When
    ``capture_stderr`` is true, returns ``(return_value, stdout_text,
    stderr_text)`` — used by the external tcllib runner to surface
    ``tcl trap: site=<id> …`` messages alongside the wasmtime
    backtrace.  The loose ``tuple`` return type keeps existing 2-tuple
    callers happy under static analysis — they unpack the first two
    slots; the stderr slot is only present when asked for explicitly.

    ``preopen_tmpdir`` — optional host directory path to grant the
    WASM instance access to, mapped to the guest path ``/``.  With
    no preopen (the default), every ``access``/``stat`` call
    returns ``ENOTCAPABLE``, so ``file exists`` etc. report
    "doesn't exist" regardless of the host filesystem state.  The
    ``TestFile`` suite passes a fresh ``pytest tmp_path`` to
    exercise the real filesystem paths.
    """
    engine = _get_engine_with_timeout() if timeout_s is not None else _get_engine()
    store = wasmtime.Store(engine)
    if memory_size is not None and memory_size > 0:
        # Cap linear-memory growth so a runaway test (e.g. an
        # ``obj.test``-style allocation loop) traps via ``memory.grow``
        # returning -1 instead of consuming the whole host's RAM.  The
        # sweep harness sizes this at a multiple of the matching
        # tclsh run's peak RSS so a "WASM is allowed to be a bit
        # bigger than the C VM" envelope is enforced.
        store.set_limits(memory_size=memory_size)
    deadline_value = 0
    if timeout_s is not None:
        # ``set_epoch_deadline`` takes an absolute counter value.  We
        # track how many ``increment_epoch`` calls we've made (across
        # all prior watchdog fires + setup increments below); the
        # next trap point is one tick beyond that.  Without the
        # tracking, a stale ``set_epoch_deadline(1)`` after the engine
        # has already advanced past 1 traps immediately and produces
        # zero output — which looked like an instantiation hang.
        global _epoch_count
        deadline_value = _epoch_count + 1
        store.set_epoch_deadline(deadline_value)
    wasi_config = wasmtime.WasiConfig()

    stdout_path = None
    if capture_stdout:
        fd, stdout_path = tempfile.mkstemp(suffix=".txt")
        os.close(fd)
        wasi_config.stdout_file = stdout_path

    stderr_path = None
    if capture_stderr:
        fd, stderr_path = tempfile.mkstemp(suffix=".err")
        os.close(fd)
        wasi_config.stderr_file = stderr_path

    if preopen_tmpdir is not None:
        # ``preopen_dir`` grants capability-bound access.  Mapping
        # the host dir to guest ``/`` means Tcl paths written
        # against the guest's ``pwd`` ("/") resolve under the host
        # path.  We pass the host path as ``dir`` (first positional)
        # and the guest path as ``guest_path``; wasmtime's Python
        # binding follows that order.
        wasi_config.preopen_dir(preopen_tmpdir, "/")

    store.set_wasi(wasi_config)

    # Instantiate Zig runtime — modules are engine-bound, so the
    # timeout variant needs its own pre-compiled copy.
    rt_module = _get_rt_module_with_timeout() if timeout_s is not None else _get_rt_module()
    linker = wasmtime.Linker(engine)
    linker.define_wasi()

    tcl_instance_box, memory_box, rt_instance_box = _define_call_compiled_proc(
        linker, store, host_spawn=host_spawn
    )

    rt_instance = linker.instantiate(store, rt_module)
    rt_instance_box[0] = rt_instance

    # Apply the requested capability mask.  Default is 0 — sandboxed
    # posture refuses ``exec`` / ``exit`` / ``glob``.  Tests that
    # need those primitives pass ``capabilities=tcl.CAP_*`` and a
    # matching ``host_spawn=`` callback (for ``CAP_EXEC``).
    if capabilities:
        set_caps_fn = rt_instance.exports(store).get("tcl_set_capabilities")
        if set_caps_fn is None:
            # An old runtime build without the capability bitset
            # would silently leave the sandboxed posture in place
            # and the test would diagnose the resulting refusal as
            # a Tcl-level capability denial — wrong root cause.
            # Surface the runtime-mismatch directly.
            raise RuntimeError(
                "runtime does not export 'tcl_set_capabilities'; "
                f"cannot apply requested capability mask {capabilities!r}"
            )
        set_caps_fn(store, capabilities)

    # WASI reactor initialisation — wasi-libc installs its ctors
    # (preopen-fd scanner, global locks, etc.) in ``_initialize``
    # rather than at instantiation.  Calling it once per store
    # populates the ``__wasilibc_cwd`` / preopen table so
    # subsequent ``access``/``stat`` calls can resolve paths
    # against the configured ``preopen_dir``.  If the runtime
    # doesn't export ``_initialize`` (old build), this is a no-op.
    init_fn = rt_instance.exports(store).get("_initialize")
    if init_fn is not None:
        init_fn(store)

    # Re-export under "tcl" namespace
    for export in rt_module.exports:
        name = export.name
        if name.startswith("__"):
            continue
        val = rt_instance.exports(store)[name]
        if isinstance(val, wasmtime.Func):
            linker.define(store, "tcl", name, val)
        elif name == "memory":
            linker.define(store, "tcl", name, val)

    # Instantiate compiled Tcl module
    tcl_module = wasmtime.Module(engine, wasm_bytes)
    tcl_instance = linker.instantiate(store, tcl_module)
    tcl_instance_box[0] = tcl_instance
    memory_box[0] = rt_instance.exports(store)["memory"]

    obj_new_int = rt_instance.exports(store)["obj_new_int"]
    obj_get_int = rt_instance.exports(store)["obj_get_int"]

    func = tcl_instance.exports(store).get(func_name)
    if func is None:
        raise RuntimeError(f"function {func_name} not found in WASM exports")

    boxed_args = tuple(obj_new_int(store, a) for a in args)
    watchdog = None
    watchdog_fired = [False]
    if timeout_s is not None:
        import threading

        def _bump_epoch():
            engine.increment_epoch()
            watchdog_fired[0] = True

        watchdog = threading.Timer(timeout_s, _bump_epoch)
        watchdog.daemon = True
        watchdog.start()
    try:
        result_obj = func(store, *boxed_args)
        result_val = obj_get_int(store, result_obj) if result_obj else 0
    except BaseException:
        # Drain captured stderr/stdout before re-raising so the trap
        # site message written by the runtime is visible to the
        # caller.  Attach unconditionally — earlier the attach was
        # gated on ``stderr_text`` being non-empty, which silently
        # discarded captures whenever a test trapped while writing
        # only to stdout (e.g. tcltest's per-test FAILED markers go
        # to stdout, but a watchdog-driven epoch trap leaves stderr
        # empty).  Result: callers got back ``""`` and thought the
        # bundle had hung from instantiation, when in fact it had
        # produced hundreds of lines of stdout before the trap.
        stdout_text = _maybe_read(stdout_path)
        stderr_text = _maybe_read(stderr_path)
        import sys

        exc = sys.exc_info()[1]
        if exc is not None:
            try:
                exc.tcl_stdout = stdout_text  # type: ignore[attr-defined]
                exc.tcl_stderr = stderr_text  # type: ignore[attr-defined]
            except (AttributeError, TypeError):
                pass
        raise
    finally:
        if watchdog is not None:
            watchdog.cancel()
            # ``_epoch_count`` mirrors the engine's epoch counter so
            # subsequent ``set_epoch_deadline`` values are always
            # strictly ahead of the live counter.  Only the watchdog
            # path advances the engine via ``increment_epoch``; a
            # clean wasm return leaves the engine's counter where it
            # was, so we do nothing here in that case.  Bumping on a
            # clean return would fabricate ticks the engine never
            # observed and eventually push deadlines past what a
            # single ``increment_epoch`` can reach.
            if watchdog_fired[0]:
                _epoch_count_set(deadline_value)

    stdout_text = _maybe_read(stdout_path)
    stderr_text = _maybe_read(stderr_path)

    if capture_stderr:
        return result_val, stdout_text, stderr_text
    return result_val, stdout_text


def _define_call_compiled_proc(
    linker: wasmtime.Linker,
    store: wasmtime.Store,
    host_spawn=None,
) -> tuple[list, list, list]:
    """Register ``env.call_compiled_proc`` on *linker*.

    The host bridge that lets the runtime dispatch a compiled proc
    by name when an interpreter-path call site encounters one.  The
    returned boxes must be filled by the caller AFTER instantiating
    the compiled user module so the callback can see it:

        tcl_instance_box[0] = tcl_instance
        memory_box[0] = rt_instance.exports(store)["memory"]

    Using a list as a 1-slot mutable cell keeps the closure trivially
    updatable without juggling ``nonlocal`` declarations in multiple
    call sites.
    """
    tcl_instance_box: list = [None]
    memory_box: list = [None]

    def _call_compiled_proc(name_ptr: int, name_len: int, argv_ptr: int, argc: int) -> int:
        # Silent 0-returns would mask genuine dispatch failures
        # (the runtime treats 0 as a successful empty-string
        # result).  For unrecoverable failures — unmapped memory,
        # missing proc, compiled proc trapping — raise so wasmtime
        # propagates a trap to the caller instead of returning a
        # phantom empty value.
        inst = tcl_instance_box[0]
        mem = memory_box[0]
        if inst is None or mem is None:
            raise RuntimeError("call_compiled_proc: tcl_instance or memory not yet wired")
        raw = bytes(mem.data_ptr(store)[name_ptr : name_ptr + name_len])
        pname = raw.decode("utf-8", errors="replace")
        func = inst.exports(store).get(pname)
        if func is None:
            raise RuntimeError(f"call_compiled_proc: compiled module does not export {pname!r}")
        # argv is a contiguous array of ``argc`` i32 TclObj pointers;
        # these are unsigned 32-bit values (WASM memory addresses /
        # TclObj pointers).  Decoding with ``signed=False`` preserves
        # the high bit; passing a negative int to ``obj_ensure_string``
        # in the runtime would trip on ``@ptrFromInt`` bounds checks.
        args: list[int] = []
        for i in range(argc):
            off = argv_ptr + i * 4
            b = bytes(mem.data_ptr(store)[off : off + 4])
            args.append(int.from_bytes(b, "little", signed=False))
        # A compiled proc that traps raises a wasmtime.Trap — let it
        # propagate so the trap carries its full backtrace + the
        # runtime's stderr diag output.  Wrapping in ``try: … except:
        # return 0`` would hide real failures.
        result = func(store, *args)
        return int(result) if result is not None else 0

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
        _call_compiled_proc,
    )

    # The runtime always declares ``env.host_spawn`` because
    # ``cmds/exec.zig`` references it from the (always-kept) BUILTIN
    # eval_exec handler.  Wire a default stub here so non-cap tests
    # instantiate cleanly; tests that opt in to ``CAP_EXEC`` pass a
    # real callback through ``host_spawn=`` and the cap-aware wrapper
    # in ``_run_wasm`` redefines the import via :func:`_define_host_spawn`
    # before instantiation.
    rt_instance_box: list = [None]
    _define_host_spawn(linker, store, memory_box, rt_instance_box, host_spawn)

    # Asyncify-enabled runtime builds (Stage 2 coroutines) declare the
    # five ``asyncify_*`` control functions as ``env`` imports in
    # ``runtime/zig/sched/tcl_asyncify.zig``.  ``wasm-opt --asyncify``
    # also exports the same names as the auto-generated
    # implementations.  The host trampolines below route imports →
    # exports of the same instance so internal callers (the coroutine
    # driver) actually trigger the unwind / rewind machinery.  When
    # the runtime is built without ``-Dasyncify=true`` no module
    # imports the ``env.asyncify_*`` symbols; the linker holds the
    # unused trampoline definitions inert.

    def _make_async_trampoline(name: str, has_arg: bool, has_result: bool):
        def _call(*py_args: int) -> int | None:
            inst = rt_instance_box[0]
            if inst is None:
                raise RuntimeError(f"asyncify trampoline {name}: rt_instance not yet wired")
            fn = inst.exports(store).get(name)
            if fn is None:
                raise RuntimeError(f"asyncify trampoline {name}: export not present")
            return fn(store, *py_args) if py_args else fn(store)

        return _call

    _i32 = wasmtime.ValType.i32()
    _arg_i32 = wasmtime.FuncType([_i32], [])
    _arg_none = wasmtime.FuncType([], [])
    _result_i32 = wasmtime.FuncType([], [_i32])
    for fname, ftype in (
        ("asyncify_start_unwind", _arg_i32),
        ("asyncify_stop_unwind", _arg_none),
        ("asyncify_start_rewind", _arg_i32),
        ("asyncify_stop_rewind", _arg_none),
        ("asyncify_get_state", _result_i32),
    ):
        try:
            linker.define_func(
                "env",
                fname,
                ftype,
                _make_async_trampoline(
                    fname,
                    has_arg=(ftype is _arg_i32),
                    has_result=(ftype is _result_i32),
                ),
            )
        except wasmtime.WasmtimeError:
            # Linker rejects redefinitions — only relevant if the test
            # harness runs the same function across runs in one Store.
            pass
    return tcl_instance_box, memory_box, rt_instance_box


def _define_host_spawn(
    linker: "wasmtime.Linker",
    store: "wasmtime.Store",
    memory_box: list,
    rt_instance_box: list,
    host_spawn,
) -> None:
    """Register ``env.host_spawn`` on *linker*.

    ``host_spawn`` is the host-imported subprocess primitive
    documented in ``docs/design/compiler/wasm-runtime-primitives.md``.
    Every compiled WASM module that includes the runtime declares
    the import, so the linker MUST satisfy it even when
    ``CAP_EXEC`` is never granted; without the definition the
    instantiation fails before the runtime gets a chance to refuse
    the call via the capability gate.

    The default stub (``host_spawn=None``) raises a wasmtime trap
    matching the runtime's "permission denied" semantics so a test
    that forgets to pass a real callback fails loudly rather than
    silently returning empty strings.  Tests that opt in to ``exec``
    pass a real Python callable that receives the decoded
    ``(argv, stdin)`` tuple and returns either:

      - a Python ``bytes``/``str`` value (interpreted as captured
        stdout) — the harness wraps it in a TclObj string and
        returns the handle, or
      - ``None`` — the harness raises a Tcl error
        ``host_spawn: refused`` through the runtime so ``catch``
        observes a normal error return.
    """

    def _call(argv_ptr: int, argv_len: int, stdin_ptr: int, stdin_len: int) -> int:
        rt_inst = rt_instance_box[0]
        mem = memory_box[0]
        if rt_inst is None or mem is None:
            raise RuntimeError("host_spawn called before runtime is wired")
        if host_spawn is None:
            raise RuntimeError("host_spawn: not configured (CAP_EXEC granted without callback)")
        argv_bytes = bytes(mem.data_ptr(store)[argv_ptr : argv_ptr + argv_len])
        # Strip the trailing NUL added by the runtime's serialiser
        # before splitting so we don't end up with an empty last
        # entry.
        if argv_bytes.endswith(b"\x00"):
            argv_bytes = argv_bytes[:-1]
        argv = [s.decode("utf-8", errors="replace") for s in argv_bytes.split(b"\x00")]
        stdin_text = ""
        if stdin_len > 0:
            stdin_bytes = bytes(mem.data_ptr(store)[stdin_ptr : stdin_ptr + stdin_len])
            if stdin_bytes.endswith(b"\x00"):
                stdin_bytes = stdin_bytes[:-1]
            stdin_text = stdin_bytes.decode("utf-8", errors="replace")
        result = host_spawn(argv, stdin_text)
        if result is None:
            return 0
        if isinstance(result, str):
            result = result.encode("utf-8")
        if not isinstance(result, (bytes, bytearray)):
            raise TypeError(
                f"host_spawn callback returned {type(result)!r}, expected str/bytes/None"
            )
        # Stage the bytes into linear memory via the runtime's
        # ``alloc`` + ``obj_new_string`` pair so the returned TclObj
        # is shaped the way the rest of the runtime expects.
        alloc_fn = rt_inst.exports(store)["alloc"]
        obj_new_string_fn = rt_inst.exports(store)["obj_new_string"]
        n = len(result)
        if n == 0:
            return int(obj_new_string_fn(store, 0, 0))
        addr = int(alloc_fn(store, n))
        # ``mem.data_ptr`` returns a ctypes pointer that doesn't
        # accept slice assignment; copy byte-by-byte instead.
        ptr = mem.data_ptr(store)
        for i in range(n):
            ptr[addr + i] = result[i]
        return int(obj_new_string_fn(store, addr, n))

    try:
        linker.define_func(
            "env",
            "host_spawn",
            wasmtime.FuncType(
                [
                    wasmtime.ValType.i32(),
                    wasmtime.ValType.i32(),
                    wasmtime.ValType.i32(),
                    wasmtime.ValType.i32(),
                ],
                [wasmtime.ValType.i32()],
            ),
            _call,
        )
    except wasmtime.WasmtimeError:
        # Linker rejects redefinitions inside one Linker; matches
        # the existing pattern used by the asyncify trampolines.
        pass


def _maybe_read(path: str | None) -> str:
    """Read and delete *path* if set, else return empty string.

    Reads as bytes and decodes with ``errors="replace"`` — the
    runtime's stderr may contain garbage bytes when a trap fires in
    the middle of a multi-byte sequence (or when memory corruption
    leaks through the error formatter), and we don't want the
    harness to crash before surfacing the trap itself.
    """
    if not path:
        return ""
    try:
        return Path(path).read_bytes().decode("utf-8", errors="replace")
    finally:
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass


def _resolve_trap(trap_exc: Exception, stderr_text: str, diag: DiagMap) -> str:
    """Decode a wasmtime trap + stderr into a human-readable message.

    Looks for ``tcl trap: site=<id> <msg>`` in *stderr_text* and
    resolves ``<id>`` against *diag* to print the originating source
    location, command, and (if known) proc.  The wasmtime backtrace
    is rewritten so ``<wasm function N>`` tokens become proc names
    when the map knows them.
    """
    import re

    out = []
    m = re.search(r"tcl trap:\s*site=(\d+)\s*(.*)", stderr_text)
    if m:
        site_id = int(m.group(1))
        msg = m.group(2).strip()
        site = next((s for s in diag.sites if s.id == site_id), None)
        if site is not None:
            out.append(f"tcl trap: {msg}")
            out.append(
                f"  at {site.file}:{site.line}:{site.col} "
                f"in {site.command} (site #{site.id}, {site.kind})"
            )
            if site.proc:
                out.append(f"  inside proc {site.proc}")
            if site.args:
                preview = ", ".join(repr(a)[:60] for a in site.args[:3])
                out.append(f"  args: {preview}")
        # ``  in eval-script at offset N: <snippet>`` is emitted by
        # the runtime when the trap fires from inside a ``tcl_eval``
        # fallback — it tells us which *command inside the fallback
        # script* actually failed, which usually matters more than
        # the site-id pointing at the fallback's creation point.
        ctx = re.search(r"^  in eval-script at offset \d+: .*$", stderr_text, flags=re.MULTILINE)
        if ctx:
            out.append(ctx.group(0))
    elif stderr_text.strip():
        out.append(f"stderr: {stderr_text.strip()}")

    # Resolve wasm function indices in the backtrace.
    trap_txt = str(trap_exc)
    func_map = {idx: qname for idx, qname in diag.procs}

    def _sub(mo: re.Match) -> str:
        idx = int(mo.group(1))
        qname = func_map.get(idx)
        return f"<{qname or f'wasm function {idx}'}>"

    trap_txt = re.sub(r"<wasm function (\d+)>", _sub, trap_txt)
    out.append(trap_txt)
    return "\n".join(out)


def _try_compile(source: str) -> tuple[bool, str]:
    """Try to compile source. Returns (success, error_message)."""
    try:
        _compile_tcl(source)
        return True, ""
    except Exception as e:
        return False, str(e)


def _try_compile_and_run(source: str, capture_stdout: bool = False):
    """Try to compile+run. Returns (success, value, stdout, error)."""
    try:
        wasm_bytes = _compile_tcl(source)
        val, stdout = _run_wasm(wasm_bytes, capture_stdout=capture_stdout)
        return True, val, stdout, ""
    except Exception as e:
        return False, 0, "", str(e)


def _run_tcl_for_value(source: str) -> tuple[bool, int, str]:
    """Wrap source in a proc, compile, and run to get an integer return value.

    The source should end with an expression or `return $val` that
    evaluates to an integer. Returns (success, value, error).
    """
    try:
        wasm_bytes, proc_name = _compile_tcl_as_proc(source)
        val, _ = _run_wasm(wasm_bytes, func_name=proc_name)
        return True, val, ""
    except Exception as e:
        return False, 0, str(e)


def _run_tcl_for_stdout(source: str) -> tuple[bool, str, str]:
    """Compile and run source, capturing stdout. Returns (success, stdout, error)."""
    try:
        wasm_bytes = _compile_tcl(source)
        _, stdout = _run_wasm(wasm_bytes, capture_stdout=True)
        return True, stdout, ""
    except Exception as e:
        return False, "", str(e)


# -- Tests for bytecode snippets (no-puts, check return value) --


# Snippets known to exercise features we deliberately don't support in the
# WASM codegen yet.  Listed here so failures on them are tracked (xfail)
# rather than hidden by a blanket skip; removing an entry turns the snippet
# back into a hard assertion.
_KNOWN_UNSUPPORTED_SNIPPETS: frozenset[str] = frozenset(
    {
        # Populate as specific known-broken snippets are identified.  A strict
        # assert is the default so regressions in previously-working snippets
        # surface immediately.
    }
)


class TestSnippetCompilation:
    """Verify that bytecode snippets compile to WASM without errors.

    Uses a strict assert by default; if a snippet is known to exercise
    unsupported features, add its stem to ``_KNOWN_UNSUPPORTED_SNIPPETS``
    so it becomes an xfail (still tracked, won't silently hide a fix
    either).  A blanket skip-on-error would let real regressions slip
    through, so we don't do that.
    """

    @pytest.mark.parametrize(
        "snippet",
        sorted(_SNIPPETS_DIR.glob("*.tcl")),
        ids=lambda p: p.stem,
    )
    def test_snippet_compiles(self, snippet: Path):
        """Each snippet should compile to valid WASM."""
        source = snippet.read_text()
        ok, err = _try_compile(source)
        if not ok and snippet.stem in _KNOWN_UNSUPPORTED_SNIPPETS:
            pytest.xfail(f"known-unsupported snippet: {err[:120]}")
        assert ok, f"snippet {snippet.stem} failed to compile: {err[:200]}"


class TestSnippetExecution:
    """Run compilable snippets and verify they don't trap."""

    SUPPORTED_SNIPPETS = [
        "01_set_simple",
        "02_set_get",
        "03_expr_braced",
        "04_expr_vars",
        "05_if_simple",
        "06_if_else",
        "07_if_elseif",
        "08_while",
        "09_for",
        "10_foreach",
        "11_proc_simple",
    ]

    @pytest.mark.parametrize("stem", SUPPORTED_SNIPPETS)
    def test_snippet_runs(self, stem: str):
        """Snippet should compile and run without trapping."""
        path = _SNIPPETS_DIR / f"{stem}.tcl"
        if not path.exists():
            pytest.skip(f"{path} not found")
        ok, val, stdout, err = _try_compile_and_run(path.read_text())
        assert ok, f"failed: {err[:200]}"


# -- Tests for fixture files (with puts, check stdout) --


class TestFixtureSimple:
    """Run simple.tcl fixture — basic variable assignment and puts."""

    def test_compiles(self):
        source = (_FIXTURES_DIR / "simple.tcl").read_text()
        ok, err = _try_compile(source)
        assert ok, f"compile error: {err}"

    def test_runs_and_outputs(self):
        source = (_FIXTURES_DIR / "simple.tcl").read_text()
        ok, stdout, err = _run_tcl_for_stdout(source)
        assert ok, f"error: {err}"
        assert "Hello, World!" in stdout
        # String interpolation: "Sum is $sum" should resolve sum=30
        assert "Sum is 30" in stdout


class TestFixtureProcs:
    """Run procs.tcl fixture — fibonacci, factorial, break/continue."""

    def test_compiles(self):
        source = (_FIXTURES_DIR / "procs.tcl").read_text()
        ok, err = _try_compile(source)
        assert ok, f"compile error: {err}"

    def test_runs_and_outputs(self):
        """Full fixture run: fib(10)=55, 10!=3628800, loop skips 5 and breaks at 15."""
        source = (_FIXTURES_DIR / "procs.tcl").read_text()
        ok, stdout, err = _run_tcl_for_stdout(source)
        assert ok, f"error: {err}"
        assert "fib(10) = 55" in stdout
        assert "10! = 3628800" in stdout
        # Loop should print i = 1..14 with i = 5 skipped, i = 15 breaks
        assert "i = 4" in stdout
        assert "i = 5" not in stdout  # continue at 5
        assert "i = 14" in stdout
        assert "i = 15" not in stdout  # break at 15

    def test_fib_proc_directly(self):
        """Call the compiled fib proc directly."""
        source = (_FIXTURES_DIR / "procs.tcl").read_text()
        wasm_bytes = _compile_tcl(source)
        val, _ = _run_wasm(wasm_bytes, func_name="::fib", args=(10,))
        assert val == 55

    def test_factorial_proc_directly(self):
        """Call the compiled factorial proc directly."""
        source = (_FIXTURES_DIR / "procs.tcl").read_text()
        wasm_bytes = _compile_tcl(source)
        val, _ = _run_wasm(wasm_bytes, func_name="::factorial", args=(10,))
        assert val == 3628800


# -- Inline real Tcl tests --


class TestRealTclInline:
    """Hand-written Tcl programs that exercise multiple features together.

    Tests that check integer return values wrap the code in a proc
    via _run_tcl_for_value (the compiled top-level returns a WASM local,
    not a TclObj, so direct obj_get_int doesn't work — procs return
    properly boxed TclObj values via the return instruction).
    """

    def test_fibonacci_proc(self):
        source = """\
proc fib {n} {
    if {$n <= 1} { return $n }
    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
}
return [fib 10]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 55

    def test_factorial_loop(self):
        source = """\
proc factorial {n} {
    set result 1
    for {set i 1} {$i <= $n} {incr i} {
        set result [expr {$result * $i}]
    }
    return $result
}
return [factorial 10]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 3628800

    def test_while_break(self):
        source = """\
set i 0
while {$i < 20} {
    incr i
    if {$i == 15} { break }
}
return $i
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 15

    def test_while_break_continue(self):
        """break and continue inside if inside while — sum 1..14 minus 5 = 100."""
        source = """\
set sum 0
set i 0
while {$i < 20} {
    incr i
    if {$i == 5} { continue }
    if {$i == 15} { break }
    set sum [expr {$sum + $i}]
}
return $sum
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 100

    def test_for_break_in_if(self):
        """break inside if inside for loop — must exit cleanly."""
        source = """\
set last 0
for {set i 0} {$i < 100} {incr i} {
    set last $i
    if {$i == 7} { break }
}
return $last
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 7

    def test_for_loop(self):
        """For loop runs to completion and accumulates correctly."""
        source = """\
set sum 0
for {set i 1} {$i <= 10} {incr i} {
    set sum [expr {$sum + $i}]
}
return $sum
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 55

    def test_string_length(self):
        source = """\
set s "Hello World"
return [string length $s]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 11

    def test_list_length(self):
        source = """\
set lst {a b c d e}
return [llength $lst]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 5

    def test_dict_get(self):
        source = """\
set d [dict create]
dict set d name Alice
dict set d age 30
return [dict get $d age]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 30

    def test_nested_proc_calls(self):
        source = """\
proc double {x} { return [expr {$x * 2}] }
proc quadruple {x} { return [double [double $x]] }
return [quadruple 7]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 28

    def test_for_loop_accumulator(self):
        source = """\
set sum 0
for {set i 1} {$i <= 100} {incr i} {
    set sum [expr {$sum + $i}]
}
return $sum
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 5050

    def test_foreach_sum(self):
        source = """\
proc sum_list {lst} {
    set total 0
    foreach item $lst {
        set total [expr {$total + $item}]
    }
    return $total
}
return [sum_list {1 2 3 4 5}]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 15

    def test_catch_error(self):
        source = "return [catch { error {boom} }]\n"
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1

    def test_switch_dispatch(self):
        source = """\
set x "hello"
switch $x {
    hello { set result 1 }
    world { set result 2 }
    default { set result 0 }
}
return $result
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1


class TestUpvarAndVariable:
    """Tests for ``upvar`` and ``variable`` alias semantics.

    ``upvar #0 target local`` aliases local → global ``target``; reads
    and writes of ``local`` route through the global table, so changes
    made from one proc are visible from another.  ``variable name``
    inside a namespace proc aliases ``name`` → ``::ns::name``.
    """

    def test_upvar_hash0_literal_target(self):
        """upvar #0 with a literal target name — writes propagate across procs."""
        source = """\
proc set_value {v} {
    upvar #0 my_global g
    set g $v
}
proc get_value {} {
    upvar #0 my_global g
    return $g
}
proc main {} {
    set_value 42
    return [get_value]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 42

    def test_upvar_hash0_dynamic_target(self):
        """upvar #0 with an interpolated target (counter::T-$tag pattern)."""
        source = """\
proc put {tag value} {
    upvar #0 store::slot-$tag s
    set s $value
}
proc fetch {tag} {
    upvar #0 store::slot-$tag s
    return $s
}
proc main {} {
    put 7 111
    put 9 222
    set a [fetch 7]
    set b [fetch 9]
    return [expr {$a + $b}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 333

    def test_upvar_incr_through_alias(self):
        """incr on an upvar'd variable updates the underlying global."""
        source = """\
proc bump {tag} {
    upvar #0 counter::c-$tag cnt
    incr cnt
}
proc fetch_cnt {tag} {
    upvar #0 counter::c-$tag cnt
    return $cnt
}
proc main {} {
    upvar #0 counter::c-x cnt
    set cnt 10
    bump x
    bump x
    bump x
    return [fetch_cnt x]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 13

    def test_variable_in_namespace_proc(self):
        """variable in a namespace proc aliases local → ::ns::local.

        We initialise the namespace var via the proc-level ``variable total 0``
        form rather than the namespace-eval-level ``variable total 0`` form:
        the interpreter's current ``namespace eval`` fallback does not yet
        execute ``variable`` bodies, so top-level initialisation doesn't
        land in the global table.  A proc-level ``variable name value``
        does emit an initialising write through our alias machinery.
        """
        source = """\
set ::ctr::total 0
proc ::ctr::add {n} {
    variable total
    set total [expr {$total + $n}]
    return $total
}
proc main {} {
    ::ctr::add 5
    ::ctr::add 7
    return [::ctr::add 3]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 15

    def test_variable_and_upvar_interop(self):
        """A namespace variable is visible via upvar #0 in another proc."""
        source = """\
set ::st::count 0
proc ::st::inc {} {
    variable count
    incr count
    return $count
}
proc main {} {
    ::st::inc
    ::st::inc
    upvar #0 ::st::count c
    return $c
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 2

    def test_variable_with_initializer(self):
        """``variable name value`` inside a proc initialises the namespace var."""
        source = """\
proc ::ns::init {} {
    variable counter 100
    return $counter
}
proc ::ns::read_counter {} {
    variable counter
    return $counter
}
proc main {} {
    ::ns::init
    return [::ns::read_counter]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 100


class TestInfoExists:
    """``info exists`` wired into WASM codegen.

    The test harness wraps each source in ``proc __main__``, so the
    ``main`` proc here is a nested proc and variables declared inside
    ``main`` are its locals.  ``::``-qualified names and upvar aliases
    route through the global table.
    """

    def test_info_exists_plain_local_after_set(self):
        source = """\
proc checker {} {
    set x 7
    if {[info exists x]} { return 1 }
    return 0
}
return [checker]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1

    def test_info_exists_global_qualified(self):
        source = """\
proc checker {} {
    if {[info exists ::myglob]} { return 1 }
    return 0
}
proc setter {} {
    set ::myglob 99
}
proc main {} {
    setter
    return [checker]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1

    def test_info_exists_missing_global(self):
        source = """\
proc checker {} {
    if {[info exists ::nope]} { return 1 }
    return 0
}
return [checker]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 0

    def test_info_exists_via_upvar_alias(self):
        """info exists on an upvar'd local checks the aliased global."""
        source = """\
proc probe {tag} {
    upvar #0 store::slot-$tag v
    if {[info exists v]} { return 1 }
    return 0
}
proc writer {tag} {
    upvar #0 store::slot-$tag v
    set v 1
}
proc main {} {
    set a [probe 1]
    writer 1
    set b [probe 1]
    return [expr {$a * 10 + $b}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        # Before write: 0; after write: 1 → 0*10 + 1 = 1
        assert val == 1

    def test_info_exists_array_element(self):
        """info exists arr(key) probes the array-element table."""
        source = """\
proc main {} {
    set a(1) 10
    set r 0
    if {[info exists a(1)]} { set r [expr {$r + 1}] }
    if {[info exists a(2)]} { set r [expr {$r + 10}] }
    return $r
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1  # only a(1) exists


class TestLassign:
    """``lassign`` destructures a list into variables and returns the rest."""

    def test_lassign_basic(self):
        source = """\
proc main {} {
    lassign {10 20 30} a b c
    return [expr {$a + $b + $c}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 60

    def test_lassign_fewer_vars_than_elements(self):
        """Extra list elements are returned as the leftover list."""
        source = """\
proc main {} {
    set rest [lassign {1 2 3 4 5} a b]
    return [llength $rest]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 3

    def test_lassign_more_vars_than_elements(self):
        """Missing elements bind to empty string; length of empty is 0."""
        source = """\
proc main {} {
    lassign {42} a b c
    return [string length $b]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 0

    def test_lassign_into_upvar_alias(self):
        """Writes go through upvar aliases — target reflects across procs."""
        source = """\
proc dispatch {tag} {
    upvar #0 ::slot::$tag s
    lassign {7 42} _ s
}
proc fetch {tag} {
    upvar #0 ::slot::$tag s
    return $s
}
proc main {} {
    dispatch abc
    return [fetch abc]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 42


class TestClock:
    """``clock seconds`` / ``clock clicks`` / ``clock milliseconds`` via WASI."""

    def test_clock_seconds_is_positive(self):
        source = """\
proc main {} {
    set t [clock seconds]
    if {$t > 0} { return 1 }
    return 0
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1

    def test_clock_clicks_is_positive(self):
        source = """\
proc main {} {
    set t [clock clicks]
    if {$t > 0} { return 1 }
    return 0
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1

    def test_clock_clicks_monotonic(self):
        """Two consecutive clicks: the second is >= the first."""
        source = """\
proc main {} {
    set a [clock clicks]
    set b [clock clicks]
    if {$b >= $a} { return 1 }
    return 0
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1

    def test_clock_milliseconds_present(self):
        source = """\
proc main {} {
    set t [clock milliseconds]
    if {$t > 0} { return 1 }
    return 0
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1


class TestArrays:
    """Tcl arrays — ``set arr(key) val``, ``$arr(key)``, ``array …``."""

    def test_array_basic_set_and_get(self):
        source = """\
proc main {} {
    set a(1) 10
    set a(2) 20
    return [expr {$a(1) + $a(2)}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 30

    def test_array_overwrite(self):
        source = """\
proc main {} {
    set a(key) 1
    set a(key) 42
    return $a(key)
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 42

    def test_array_incr(self):
        source = """\
proc main {} {
    set counter(N) 0
    incr counter(N)
    incr counter(N)
    incr counter(N)
    return $counter(N)
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 3

    def test_array_dynamic_key(self):
        """Keys can be computed at runtime — use $var in key position."""
        source = """\
proc main {} {
    set key alpha
    set a($key) 7
    set a(beta) 11
    return [expr {$a($key) + $a(beta)}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 18

    def test_array_exists_size(self):
        source = """\
proc main {} {
    set r 0
    if {[array exists missing]} { set r [expr {$r + 100}] }
    set a(1) 1
    set a(2) 1
    if {[array exists a]} { set r [expr {$r + 1}] }
    set r [expr {$r + [array size a]}]
    return $r
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 3  # 0 missing-hit + 1 a-exists + 2 size

    def test_array_names_join(self):
        source = """\
proc main {} {
    set a(x) 1
    set a(y) 2
    set a(z) 3
    set names [array names a]
    return [llength $names]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 3

    def test_array_unset_element(self):
        source = """\
proc main {} {
    set a(1) 1
    set a(2) 2
    unset a(1)
    return [array size a]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1

    def test_array_unset_whole(self):
        source = """\
proc main {} {
    set a(1) 1
    set a(2) 2
    array unset a
    return [array size a]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 0

    def test_array_exists_after_removing_all_elements(self):
        """``array exists`` returns 1 for an array variable even when it
        has zero elements — matches Tcl semantics where the array
        variable persists after the last ``unset``.
        """
        source = """\
proc main {} {
    set a(1) 1
    unset a(1)
    if {[array exists a]} { return 1 }
    return 0
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 1

    def test_array_read_in_puts_value_context(self):
        """``$arr(key)`` reads outside expression context — the
        codegen must dispatch through ``tcl_array_get`` rather than
        treating ``a`` as a local and ``(x)`` as literal text.
        Regression test for ``_resolve_var_name`` not accepting
        array refs.
        """
        wasm, _ = _compile_tcl_with_diag("set a(x) 42\nputs $a(x)\n")
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "42\n"

    def test_array_read_in_interpolated_string(self):
        """``"v=$arr(key)"`` — the interpolated parser must consume
        the ``(key)`` as part of the variable reference, not split
        it into a variable ``$a`` followed by literal ``(x)``.
        """
        wasm, _ = _compile_tcl_with_diag('set a(x) 42\nputs "v=$a(x) done"\n')
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "v=42 done\n"

    def test_array_read_via_set_single_arg(self):
        """``[set arr(key)]`` — the 1-arg ``set`` read form in value
        context must route through the array-element reader.
        Without the special-case in ``_emit_command_subst_value``
        it falls through to the eval fallback and returns empty.
        """
        wasm, _ = _compile_tcl_with_diag("set a(x) 42\nputs [set a(x)]\n")
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "42\n"

    def test_array_read_via_set_single_arg_expr(self):
        """``[set arr(key)]`` in expression context — mirror of the
        value-context special-case, for ``if {[set a(x)] == 1}``.
        """
        wasm, _ = _compile_tcl_with_diag(
            "set a(x) 1\nif {[set a(x)] == 1} { puts yes } else { puts no }\n"
        )
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "yes\n"

    def test_array_read_in_proc_value_context(self):
        """Array reads inside a proc must go through the same
        value-context path as at top level.
        """
        wasm, _ = _compile_tcl_with_diag("proc f {} { set a(x) 42; puts <$a(x)> }\nf\n")
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "<42>\n"

    def test_array_read_with_interpolated_key(self):
        """``$arr($i)`` — the key contains its own ``$var``
        substitution.  The interpolated-parts scanner must consume
        through the matching ``)`` rather than treating ``$i)`` as
        a separate variable followed by literal ``)``.
        """
        wasm, _ = _compile_tcl_with_diag("set i x\nset a($i) 42\nputs $a($i)\n")
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "42\n"

    def test_array_set_list_form(self):
        """``array set arr {k v k v ...}`` — the pair-list form
        must populate each element, not store the whole list under
        one key.  Regression for tcltest's ``ArrayDefault numTests
        [list Total 0 …]`` where the old interpreter path stored
        the full payload as a single key and later ``incr
        numTests(Total)`` ran on an uninitialised element.
        """
        wasm, _ = _compile_tcl_with_diag(
            'array set a {x 1 y 2 z 3}\nputs "x=$a(x) y=$a(y) z=$a(z)"\n'
        )
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "x=1 y=2 z=3\n"

    def test_array_set_list_via_command_subst(self):
        """``array set arr [list k v k v …]`` — command-sub payload
        reaches the Zig interpreter via ``_emit_eval_fallback`` (the
        compile-time list-literal path only fires on brace literals),
        which must route through the new ``array_set_list`` helper.
        """
        wasm, _ = _compile_tcl_with_diag('array set a [list x 1 y 2]\nputs "x=$a(x) y=$a(y)"\n')
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "x=1 y=2\n"

    def test_array_read_in_double_quoted_string(self):
        """``"v=$arr(key)"`` — the lowerer rewrites to
        ``v=${arr(key)}`` which the import-scan must still
        recognise as needing ``tcl_array_get``; without the braced
        scan fix, the read falls back to ``i32.const 0`` at runtime
        (silent empty-string).
        """
        wasm, _ = _compile_tcl_with_diag('set a(x) 42\nputs "v=$a(x) done"\n')
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "v=42 done\n"


class TestNamespaceVariables:
    """Variables inside ``namespace eval ::ns { … }`` bodies must
    persist at ``::ns::<name>``.  Regression coverage for the
    previously no-op ``variable`` at namespace-eval top-level and
    the bare-local fast path that mis-qualified unqualified names.
    """

    def test_variable_with_initializer_persists(self):
        src = "namespace eval ns { variable x 5 }\nputs [set ::ns::x]\n"
        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "5\n"

    def test_set_inside_namespace_eval_persists(self):
        src = "namespace eval ns { set x 5 }\nputs [set ::ns::x]\n"
        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "5\n"

    def test_incr_inside_namespace_eval(self):
        """``incr`` must route through the global table too, not
        operate on a bare local.  Otherwise the incremented value
        doesn't end up at ``::ns::n``.
        """
        src = "namespace eval ns { variable n 0; incr n; incr n }\nputs [set ::ns::n]\n"
        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "2\n"

    def test_variable_alias_in_proc_still_works(self):
        """The in-proc ``variable name`` alias path mustn't regress
        — it still needs to route reads/writes through the global
        table using the enclosing namespace.
        """
        src = (
            "namespace eval ns {\n"
            "    variable x 0\n"
            "    proc inc {} { variable x; incr x }\n"
            "    inc; inc; inc\n"
            "}\n"
            "puts [set ::ns::x]\n"
        )
        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "3\n"

    def test_array_unset_preserves_probe_chain(self):
        """Deleting an element must not break lookups of collision-mates.

        We insert enough keys to provoke multiple collisions in the
        initial 8-bucket table, unset one in the middle, and check
        that the remaining keys are all still findable.
        """
        source = """\
proc main {} {
    set n 12
    for {set i 0} {$i < $n} {incr i} {
        set a($i) $i
    }
    unset a(3)
    set sum 0
    for {set i 0} {$i < $n} {incr i} {
        if {[info exists a($i)]} {
            set sum [expr {$sum + $a($i)}]
        }
    }
    return $sum
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        # Sum 0..11 = 66, minus 3 = 63.  If the probe chain breaks
        # we'd miss some later keys and the sum drops below 63.
        assert val == 63

    def test_array_unset_and_reinsert(self):
        """After unsetting a key we can still insert a new one with the
        same or a colliding hash and look it up.
        """
        source = """\
proc main {} {
    set a(x) 100
    unset a(x)
    set a(x) 7
    set a(y) 11
    return [expr {$a(x) + $a(y)}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 18

    def test_array_set_literal(self):
        source = """\
proc main {} {
    array set a {one 1 two 2 three 3}
    return [expr {$a(one) + $a(two) + $a(three)}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 6

    def test_array_via_upvar_alias(self):
        """upvar #0 of a dynamic array name — tcllib counter pattern."""
        source = """\
proc put {tag k v} {
    upvar #0 store::T-$tag arr
    set arr($k) $v
}
proc fetch {tag k} {
    upvar #0 store::T-$tag arr
    return $arr($k)
}
proc main {} {
    put alpha N 10
    put alpha total 100
    put beta N 1
    return [expr {[fetch alpha N] + [fetch alpha total] + [fetch beta N]}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 111

    def test_array_in_namespace_proc_after_global_scalar_stamp(self):
        """Issue #273: ``set arr(idx) val`` inside a proc whose
        resolution namespace is non-root must not collide with a
        same-simple-name *global scalar* — e.g. tcltest's
        ``set errorInfo(body) ...`` after ``catch`` has stamped
        ``::errorInfo`` via ``stamp_error_globals``.

        Real Tcl keeps the namespace-scoped array variable disjoint
        from the global scalar, but our flat array directory and
        unqualified-by-default proc emission collapsed both onto the
        same key, so ``find_or_create`` raised
        ``can't set "errorInfo(body)": variable isn't array``.

        The runtime fix qualifies unqualified array names with the
        proc's current namespace before consulting the directory
        and the scalar-conflict probe — mirrors the script-level
        qualification ``_emit_array_name_obj`` already performs in
        ``namespace eval`` blocks.
        """
        src = """\
catch { error boom }
namespace eval ::myns {
    proc do_set {} {
        set errorInfo(body) hello
        return $errorInfo(body)
    }
    puts [do_set]
}
"""
        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "hello\n"

    def test_array_in_namespace_eval_after_global_scalar_stamp(self):
        """Companion to the proc case: at script-level inside
        ``namespace eval ::myns2 { ... }``, the array write must
        also bypass the global ``::errorInfo`` scalar.  Pre-fix this
        case worked because ``_emit_array_name_obj`` qualified the
        name at compile time; this regression test guards against a
        future emitter change that drops the script-level
        qualification.
        """
        src = """\
catch { error boom }
namespace eval ::myns2 {
    set errorInfo(stage) ready
    puts $errorInfo(stage)
}
"""
        wasm, _ = _compile_tcl_with_diag(src)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "ready\n"


class TestUplevel:
    """``uplevel`` runs a script in a caller's scope.

    Compiled procs don't currently push frames, so in a fully-compiled
    call chain ``uplevel 1`` and ``uplevel #0`` both resolve to global
    scope — matching ``#0`` semantics.  Scripts that need true
    caller-frame semantics require a caller that's running through the
    interpreter (which does push a frame).
    """

    def test_uplevel_hash0_set_global(self):
        """uplevel #0 {set X V} — writes the global X."""
        source = """\
proc setup {} {
    uplevel #0 {set ::g 42}
}
proc main {} {
    setup
    return $::g
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 42

    def test_uplevel_hash0_complex_script(self):
        """uplevel #0 with a multi-command script that the tiny embedded
        interpreter can execute (no ``$::ns``-in-expr traps).
        """
        source = """\
proc prep {} {
    uplevel #0 {
        set ::a 10
        set ::b 20
        set ::c 30
    }
}
proc main {} {
    prep
    return [expr {$::a + $::b + $::c}]
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 60

    def test_uplevel_default_level(self):
        """``uplevel {set X V}`` with no level defaults to 1.

        In a fully-compiled chain this still degrades to global since
        no frames are pushed — sufficient for scripts that use uplevel
        to simulate macros on globals.
        """
        source = """\
proc mkglob {name val} {
    uplevel "set ::$name $val"
}
proc main {} {
    mkglob myvar 99
    return $::myvar
}
return [main]
"""
        ok, val, err = _run_tcl_for_value(source)
        assert ok, f"error: {err}"
        assert val == 99


class TestUpvarCompilation:
    """Upvar/variable compilation tests — validate without execution."""

    def test_upvar_literal_target_compiles(self):
        source = """\
proc p {} {
    upvar #0 foo x
    set x 1
}
"""
        ok, err = _try_compile(source)
        assert ok, f"compile error: {err}"

    def test_upvar_dynamic_target_compiles(self):
        source = """\
proc p {tag} {
    upvar #0 counter::T-$tag c
    set c 0
}
"""
        ok, err = _try_compile(source)
        assert ok, f"compile error: {err}"

    def test_variable_no_init_compiles(self):
        source = """\
namespace eval ::ns {
    variable v
}
proc ::ns::use {} {
    variable v
    set v 42
}
"""
        ok, err = _try_compile(source)
        assert ok, f"compile error: {err}"


class TestDiagMap:
    """Source-location diag map — codegen side of the trap resolver.

    Verifies that the codegen attaches a site entry with a correct
    (file, line, col) for every ``_emit_eval_fallback`` /
    ``_emit_unsupported_trap`` call, that the runtime emits a
    ``tcl trap: site=<id> <msg>`` line on an unknown command, and
    that ``_resolve_trap`` round-trips both the site and the proc
    name in the wasmtime backtrace.
    """

    def test_stub_dispatch_records_runtime_site(self):
        # ``encoding`` is wired to a runtime stub (tcl_fmt_stubs.zig)
        # via ``_CMD_RUNTIME``; the dispatch registers a ``runtime``
        # diag site so a trap inside the stub attributes to the
        # right source location.
        source = "encoding convertfrom identity hello\n"
        _, diag = _compile_tcl_with_diag(source, "t.tcl")
        assert len(diag.sites) >= 1
        hit = [s for s in diag.sites if s.command == "encoding"]
        assert hit, f"no 'encoding' site in diag map: {diag.sites}"
        site = hit[0]
        # encoding is now wired to a runtime stub (tcl_fmt_stubs.zig),
        # so the site kind is ``runtime`` rather than ``fallback``.
        # The stub itself traps with "unsupported command: encoding".
        assert site.kind == "runtime"
        assert site.file == "t.tcl"
        assert site.line == 1
        assert site.col == 1
        assert "convertfrom" in site.args

    def test_exec_stub_records_fallback_site(self):
        # ``exec`` is variadic, so the registry deliberately omits a
        # ``wasm_runtime_import`` (the codegen's argc=N truncation
        # would silently drop trailing argv words).  Every call now
        # routes through the eval-fallback into the BUILTIN
        # capability-gated handler in ``runtime/zig/cmds/exec.zig``.
        source = "exec /bin/true\n"
        _, diag = _compile_tcl_with_diag(source, "t.tcl")
        hit = [s for s in diag.sites if s.command == "exec"]
        assert hit, f"no 'exec' site in diag map: {diag.sites}"
        assert hit[0].kind == "fallback"

    def test_procs_indexed_by_wasm_function(self):
        # Every compiled proc (and ::top) must appear in diag.procs so
        # a ``<wasm function N>`` backtrace resolves to a proc name.
        source = "proc ::foo {} { return 1 }\nproc ::bar {} { return 2 }\n"
        _, diag = _compile_tcl_with_diag(source, "t.tcl")
        qnames = {qn for _, qn in diag.procs}
        assert "::top" in qnames
        assert "::foo" in qnames
        assert "::bar" in qnames
        # Indices must be strictly increasing (one per func).
        idxs = [f for f, _ in diag.procs]
        assert idxs == sorted(idxs)

    @pytest.mark.parametrize(
        ("source", "expected_cmd"),
        [
            # ``open`` / ``close`` / ``read`` / ``gets`` / ``seek`` /
            # ``tell`` / ``eof`` / ``fblocked`` / ``fcopy`` now have
            # WASI-backed implementations in :file:`io/tcl_chan.zig`.
            # Failure paths surface through ``stubs.raise`` with a
            # specific message (``open: could not open file``,
            # ``close: unknown channel``, …) rather than the
            # ``unsupported command:`` prefix.  See TestChannelIO
            # for the positive cases.
            # ``file`` now has real WASI-backed impls for mkdir /
            # delete / rename / copy / link / readlink / stat /
            # lstat / type — see TestFile for the positive cases.
            # Failures from those route through ``stubs.raise``
            # with a syscall-specific message rather than the
            # ``unsupported command:`` prefix, so we no longer
            # parametrize a ``file`` stub entry here.
            # ``glob`` and ``exec`` are now capability-gated BUILTINs
            # (see ``runtime/zig/interp/tcl_caps.zig``).  Without
            # ``CAP_FS_GLOB`` / ``CAP_EXEC`` granted, calls raise
            # ``permission denied`` rather than ``unsupported
            # command:`` — covered by TestCapGlob / TestCapExec in
            # ``tests/test_wasm_capabilities.py``.  Use ``load`` /
            # ``unload`` to exercise the trapping-stub path here:
            # both are filesystem/process commands without a real
            # WASM impl yet.
            ("load /tmp/foo.so\n", "load"),
            ("unload /tmp/foo.so\n", "unload"),
            # ``regexp`` and ``regsub`` have real impls — see TestRegexp
            # and the regsub tests in TestDiagMap.
            # ``format`` has a minimal real impl in tcl_format.zig —
            # see TestFormat.  ``scan`` now has a minimal real impl
            # in tcl_fmt_stubs.zig covering %c / %d / %i / %x / %o /
            # %s; unknown specifiers still trap.
            # ``source`` has a real WASI-fd-resolution impl in
            # tcl_fs.zig — invoking it with a non-existent path
            # raises ``source: file not found`` rather than the old
            # ``unsupported command: source``.  See TestSource for
            # the positive assertion.
            # ``after`` is now a no-op (silently succeeds) so that
            # counter::init -timehist can schedule without erroring in
            # our WASM event-loop-less sandbox.  See stubs_.zig.
            # encoding + fconfigure have minimal real implementations
            # (tcl_encoding.zig / tcl_chan.zig); see
            # TestEncoding / TestFconfigure for their semantics.
        ],
    )
    def test_core_stubs_trap_with_unsupported(self, source, expected_cmd):
        """Each Tcl 8.4–9.0 stub must trap with ``unsupported command: X``.

        Guards against silent-NOP regressions.  Also verifies the diag
        machinery attributes the trap to the correct site (command
        name matches, kind is ``runtime``).
        """
        wasm, diag = _compile_tcl_with_diag(source, "t.tcl")
        try:
            _run_wasm(wasm, capture_stderr=True)
            pytest.fail(f"{expected_cmd} did not trap")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert f"unsupported command: {expected_cmd}" in stderr, f"stderr was: {stderr!r}"
            hits = [s for s in diag.sites if s.command == expected_cmd]
            assert hits, f"no {expected_cmd} site in diag map"
            assert hits[0].kind == "runtime"

    def test_eval_context_snippet_on_inner_trap(self):
        """Interpreter-walked commands include a source snippet on trap.

        When a compiled fallback hands an inner script to ``tcl_eval``
        and a command inside that script traps (``encoding`` /
        ``fconfigure`` / …), the runtime writes an
        ``in eval-script at offset N: <snippet>`` line so the failing
        command can be located within the fallback, not just the
        fallback's creation site.
        """
        # A standalone ``[cmd …]`` at top level compiles as an eval
        # fallback, so the interpreter walks it command-by-command
        # and the eval-context stamping runs before each dispatch.
        # Use ``load`` — a trapping stub that's not in the
        # interpreter's implemented-command set, so the eval path
        # still exercises the unsupported-command branch.  ``exec``
        # used to live here too but is now a capability-gated
        # BUILTIN that raises ``permission denied`` instead.
        source = "[load /tmp/foo.so]\n"
        wasm, diag = _compile_tcl_with_diag(source, "t.tcl")
        try:
            _run_wasm(wasm, capture_stderr=True)
            pytest.fail("expected trap")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "unsupported command: load" in stderr
            assert "in eval-script at offset" in stderr
            # The snippet must contain at least the command name.
            assert "load" in stderr

    def test_parse_bare_tracks_bracket_nesting(self):
        """``parse_bare`` must keep ``[cmd arg]`` as a single word.

        Regression test for the pre-diag-commit "cloc" off-by-one:
        the interpreter's bare-word tokenizer used to split
        ``[clock seconds]`` on whitespace, truncating inner
        command-substitution contents to ``cloc`` (one char short
        because the closing ``]`` landed in the next word).
        """
        # Run a standalone ``[clock seconds]`` through the eval
        # fallback and check the inner unknown-command trap sees the
        # full command name, not a truncated prefix.
        source = "[xabcdef seconds]\n"
        wasm, diag = _compile_tcl_with_diag(source, "t.tcl")
        try:
            _run_wasm(wasm, capture_stderr=True)
            pytest.fail("expected trap")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            # "xabcdef" is 7 chars — previously the trap would have
            # written 6-char "xabcde" with the trailing 'f' lost to
            # the ``]``-consumed-by-next-word parse.
            assert "unknown command: xabcdef" in stderr, f"stderr was: {stderr!r}"

    def test_runtime_trap_emits_site_prefix(self):
        # End-to-end: trigger an unknown command through the eval
        # fallback and verify the stderr line is the promised shape.
        source = "definitely_not_a_command\n"
        wasm, diag = _compile_tcl_with_diag(source, "t.tcl")
        try:
            _run_wasm(wasm, capture_stderr=True)
            pytest.fail("expected trap")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "tcl trap: site=" in stderr, f"stderr was: {stderr!r}"
            # The resolver must map the site back to our source file.
            resolved = _resolve_trap(trap, stderr, diag)
            assert "t.tcl:1:1" in resolved
            assert "unknown command: definitely_not_a_command" in resolved


class TestReturnCodeError:
    """``return -code error "<msg with $substitutions>"`` — inline path.

    This special-cased codegen lets the message value go through
    ``_emit_value`` (which handles ``$var`` / ``[cmd]``
    interpolation) rather than the eval fallback's brace-quoting
    that would block substitution.  The literal test below would
    previously emit the raw ``${var}`` text because the fallback
    wraps the message in braces to preserve list structure.
    """

    def test_error_message_substitutes_vars(self):
        # Wrap the error-raising call in ``catch`` so the test
        # can observe the captured message through the resultVar.
        # Without the special-case codegen, ``msg`` would be
        # ``"ambiguous option ${o}: could match ${m}"`` (raw).
        source = (
            "proc fail {o m} {\n"
            '    return -code error "ambiguous option $o: could match $m"\n'
            "}\n"
            "if {[catch {fail -tmpdir {a b}} msg]} {\n"
            "    puts $msg\n"
            "}\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        # ``$o`` must have expanded to ``-tmpdir`` and ``$m`` to
        # ``a b``.  Literal ``${o}`` in the output = regression.
        assert "ambiguous option -tmpdir: could match a b" in stdout, f"got: {stdout!r}"


class TestArrayNamesPattern:
    """``array names arr ?pattern?`` — glob filter.

    Tcl's array names accepts an optional trailing glob pattern
    that narrows the key list (same semantics as ``string
    match``).  Previously the runtime's ``array_names`` ignored
    the pattern entirely — tcltest's ``MatchingOption`` relies on
    it to turn ``array names Option $option*`` into a prefix
    lookup.
    """

    def test_no_pattern_returns_all_keys(self):
        source = "array set a {one 1 two 2 three 3 four 4}\nputs [lsort [array names a]]\n"
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "four one three two"

    def test_literal_glob_pattern(self):
        source = "array set a {one 1 two 2 three 3 four 4}\nputs [lsort [array names a t*]]\n"
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        # Should match ``two`` and ``three``.
        assert stdout.strip() == "three two"

    def test_interpolated_glob_pattern(self):
        # Verifies ``tcl_append`` gets imported so the pattern
        # ``$prefix*`` actually interpolates rather than arriving
        # as the raw ``${prefix}*`` literal (which would match
        # nothing).
        source = (
            "array set a {-tmpdir /tmp -verbose 1 -notfile foo}\n"
            "set prefix -tmp\n"
            "puts [array names a $prefix*]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "-tmpdir"

    def test_pattern_matches_nothing(self):
        source = "array set a {foo 1 bar 2}\nputs -[array names a zzz*]-\n"
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        # Empty match → empty string between the delimiters.
        assert stdout.strip() == "--"


class TestSwitchWithCommandSubst:
    """``switch`` whose subject is a command substitution.

    The CFG builder wraps the switch subject as
    ``ExprBinary(STR_EQ, ExprRaw(subject_text), ExprLiteral(arm))``.
    ``_emit_str_value`` used to emit ``ExprRaw`` through
    ``_emit_obj_literal``, which shipped the raw text (e.g.
    ``"[llength $x]"``) instead of executing it — so the
    comparison never matched any arm and ``default`` always
    fired.  Routing ``ExprRaw`` through ``_emit_value`` honours
    command-substitution and interpolation.
    """

    def test_llength_direct_as_switch_subject(self):
        source = (
            "switch -- [llength -tmpdir] {\n"
            "    0 { puts zero }\n"
            "    1 { puts one }\n"
            "    default { puts default }\n"
            "}\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "one"

    def test_llength_of_empty_list(self):
        source = (
            "switch -- [llength {}] {\n"
            "    0 { puts zero }\n"
            "    1 { puts one }\n"
            "    default { puts default }\n"
            "}\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "zero"

    def test_interpolated_subject_matches_arm(self):
        source = (
            "set prefix hello\n"
            'switch -- "$prefix-world" {\n'
            "    hello-world { puts greet }\n"
            "    default { puts default }\n"
            "}\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "greet"


class TestInfoLevel:
    """``info level 0`` inside a compiled proc.

    Returns the proc's current invocation as a list whose first
    element is the proc name and the rest are the parameter
    values.  This unblocks tcltest's ``outputChannel`` /
    ``errorChannel`` checks (``[llength [info level 0]] == 1``
    means "called with no args") — without a working impl,
    ``llength`` sees an empty string, returns 0, and the compiled
    body falls through to the ``open $filename`` branch that traps
    on our unsupported ``open`` stub during ``cleanupTests``.
    """

    def test_info_level_zero_reports_no_args(self):
        source = (
            'proc foo {{filename ""}} {\n'
            "    if {[llength [info level 0]] == 1} {\n"
            "        puts no-args\n"
            "    } else {\n"
            "        puts with-args\n"
            "    }\n"
            "}\n"
            "foo\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "no-args"

    def test_info_level_zero_with_args(self):
        source = 'proc foo {{filename ""}} {\n    puts "l=[llength [info level 0]]"\n}\nfoo hello\n'
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "l=2"


class TestProcDefaultsInCallSubst:
    """Procs called via ``[name]`` in command-substitution context.

    ``[outputChannel]`` with no args should receive the declared
    default value (empty string for ``{{filename ""}}``), not a
    boxed-integer 0 sentinel.  Without the declared-default fix,
    ``filename`` inside ``outputChannel`` was the integer TclObj
    0, making ``[info level 0]`` report a 2-element list and
    forcing the compiled body into the ``open $filename`` default
    branch that trapped on our unsupported ``open`` stub during
    tcltest's ``cleanupTests``.
    """

    def test_default_empty_string_reaches_callee(self):
        # Use ``string length`` rather than ``eq ""``/``eq {}`` —
        # our string-equality operator has a separate gap when
        # comparing against empty-string literals that this test
        # would otherwise trip on.
        source = (
            'proc foo {{filename ""}} {\n'
            "    if {[string length $filename] == 0} {\n"
            "        puts empty\n"
            "    } else {\n"
            '        puts "value=<$filename>"\n'
            "    }\n"
            "}\n"
            "puts [foo]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert "empty" in stdout.splitlines()

    def test_default_literal_reaches_callee(self):
        source = 'proc greet {{name world}} {\n    puts "hello $name"\n}\nset msg [greet]\n'
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "hello world"


class TestVariadicArgs:
    """Tcl ``proc … args {…}`` variadic-tail support.

    When a proc's last parameter is named ``args`` Tcl treats it
    specially: all extra call arguments get packed into a list and
    bound to ``args``.  Our compiled functions have fixed-arity
    signatures, so the runtime's host-bridge dispatcher
    (``tcl_dispatch.zig``) does the packing — checks
    ``proc_get_args_tail`` and builds a list TclObj for the tail
    before invoking ``call_compiled_proc``.

    Before this support, ``proc foo {args} {}`` compiled to a
    single-i32-param WASM function, and calling ``foo a b`` from
    the interpreter (via eval fallback) trapped with ``too many
    parameters provided: given 2, expected 1`` — the blocker for
    tcltest's bundle runner whose ``useLocal counter.tcl
    counter`` stub passes 2 args to a one-param variadic proc.
    """

    def test_args_with_zero_extras(self):
        source = (
            'proc f {args} { return "args-len=[llength $args]" }\n'
            "# Call via eval so the host-bridge dispatcher runs.\n"
            "puts [eval {f}]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "args-len=0"

    def test_args_with_multiple_extras(self):
        source = 'proc f {args} { return "args-len=[llength $args]" }\nputs [eval {f a b c d}]\n'
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "args-len=4"

    def test_positional_then_args_tail(self):
        source = (
            "proc f {a b args} {\n"
            '    return "a=$a b=$b args=[llength $args]"\n'
            "}\n"
            "puts [eval {f one two three four five}]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "a=one b=two args=3"

    def test_args_element_with_whitespace(self):
        # Element with embedded spaces must stay a single list
        # element — verifies list-element quoting in
        # ``build_args_list``.  Without the ``{`` / ``}`` wrap,
        # ``llength $args`` would report 4 (the dispatcher would
        # have space-joined everything into ``hello world a b c``
        # which splits back into four words).
        source = (
            "proc f {args} { return [llength $args] }\n"
            "# ``set x [list …]; eval $x`` is the direct route\n"
            "# to the host-bridge dispatcher with multi-word\n"
            "# arguments.  The ``eval [list …]`` one-liner goes\n"
            "# through a different codegen path we can pin once\n"
            "# that's fixed (separate investigation).\n"
            'set x [list f "hello world" "a b c"]\n'
            "puts [eval $x]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert stdout.strip() == "2"


class TestRegexp:
    """Tcl regex engine (Henry Spencer) linked via ``tcl_regex.zig``.

    The engine covers Tcl's full ARE syntax — ``^``/``$`` anchors,
    alternation, groups, ``\\s``/``\\w``/``\\d`` escapes, character
    classes — because it's the same engine ``tclsh`` ships with.
    These tests pin the baseline behaviour so regressions in the
    build/shim layer surface immediately.

    Capture vars and options beyond ``-nocase`` / ``--`` are not
    yet wired up; the interpreter path silently ignores them (see
    ``tcl_regex.zig::eval_regexp_cmd``).
    """

    @pytest.mark.parametrize(
        "pattern,subject,expected",
        [
            # Basic literal match
            (r"hello", "hello world", 1),
            (r"xyz", "hello world", 0),
            # Anchors
            (r"^hello", "hello world", 1),
            (r"^hello", "world hello", 0),
            (r"world$", "hello world", 1),
            (r"world$", "world hello", 0),
            # Alternation + groups
            (r"(cat|dog|bird)", "I have a dog", 1),
            (r"(cat|dog|bird)", "I have a fish", 0),
            # ``\s``/``\d``/``\w`` Perl-style escapes (ARE feature)
            (r"\s+", "hello   world", 1),
            (r"\s+", "helloworld", 0),
            (r"\d+", "abc123def", 1),
            (r"\d+", "no digits here", 0),
            (r"\w+", "hello", 1),
            # Character classes
            (r"[0-9]+", "abc42", 1),
            (r"[0-9]+", "abcdef", 0),
            (r"[^0-9]+", "42", 0),
            # The tcltest ``AcceptVerbose`` regex — alternation with
            # anchors is the pattern that used to trap the runner
            # when ``regexp`` was a stub.
            (r"^(list|pass|body|skip|error)$", "body", 1),
            (r"^(list|pass|body|skip|error)$", "unknown", 0),
            (r"^(list|pass|body|skip|error)$", "pass", 1),
        ],
    )
    def test_regexp_match(self, pattern, subject, expected):
        # Pattern is brace-protected so Tcl's parser doesn't treat
        # bracket characters as command substitutions.
        source = (
            "proc t {} {\n"
            f"    if {{[regexp {{{pattern}}} {{{subject}}}]}} {{\n"
            "        puts MATCH\n"
            "    } else {\n"
            "        puts NOMATCH\n"
            "    }\n"
            "}\n"
            "t\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        want = "MATCH" if expected == 1 else "NOMATCH"
        assert want in stdout, f"pattern={pattern!r} subject={subject!r}: {stdout!r}"

    @pytest.mark.parametrize(
        "pattern,subject,expected",
        [
            # ``-nocase`` takes a non-2-arg shape, so the codegen
            # routes through eval fallback → interpreter's
            # ``eval_regexp_cmd`` switch parser.  Pin the option
            # parsing here to catch regressions in that path.
            (r"hello", "HELLO", 0),  # case-sensitive: no match
            (r"hello", "HELLO", 1),  # case-insensitive: match
            (r"^WORLD$", "world", 1),  # case-insensitive anchors too
        ],
    )
    def test_regexp_nocase(self, pattern, subject, expected):
        # Index 0 is case-sensitive (no -nocase), 1 and 2 are.
        # pytest parametrize orders them — so we decide via expected:
        # case-sensitive "HELLO" against "hello" yields 0 (no match),
        # with -nocase it yields 1.
        use_nocase = expected == 1
        switches = "-nocase " if use_nocase else ""
        source = (
            "proc t {} {\n"
            f"    if {{[regexp {switches}-- {{{pattern}}} {{{subject}}}]}} {{\n"
            "        puts MATCH\n"
            "    } else {\n"
            "        puts NOMATCH\n"
            "    }\n"
            "}\n"
            "t\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        want = "MATCH" if expected == 1 else "NOMATCH"
        assert want in stdout, (
            f"pattern={pattern!r} subject={subject!r} nocase={use_nocase}: {stdout!r}"
        )

    def test_regexp_invalid_pattern_raises(self):
        """``regexp`` with an unparseable pattern must raise a real
        error, not silently report no-match.  Silent failure would
        hide typos in user regexps.
        """
        # ``(`` with no closing paren is a compile-time error in
        # the Henry-Spencer engine (unbalanced subexpression).
        source = (
            "if {[catch {regexp { (abc} xyz} msg]} {\n"
            "    puts CAUGHT\n"
            "} else {\n"
            "    puts MISSED\n"
            "}\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert "CAUGHT" in stdout, stdout

    def test_regexp_options_accepted(self):
        """Phase 4.5 accepts the Tcl 9 regexp option set without
        raising "unknown option" — even when the result-shaping
        for -indices/-inline/-all is still partial.  Truly-bogus
        switches still raise so callers see typos clearly.
        """
        # Truly-unknown still raises.
        source = (
            "if {[catch {regexp -bogus {a+} aa} msg]} {\n"
            "    puts CAUGHT\n"
            "} else {\n"
            "    puts MISSED\n"
            "}\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        _, stdout, _ = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        assert "CAUGHT" in stdout, stdout


class TestSubstSemantics:
    """``subst`` must execute each ``$var`` / ``[cmd]`` exactly
    once (Tcl semantics) and size its output buffer against the
    actual — not pre-computed and re-computed — substitution
    lengths.  Regression coverage for the previous two-pass
    implementation that called ``eval_script`` twice per
    substitution, doubling side effects and reopening the
    overflow class the two-pass approach was meant to close.
    """

    def test_subst_command_runs_once(self):
        source = 'set x 0\nset r [subst {[incr x]}]\nputs "r=$r"\n'
        wasm, _ = _compile_tcl_with_diag(source)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        # subst returns the result of a single incr — "1".  Two
        # executions would return "2" (second incr returns 2).
        assert stdout == "r=1\n"

    def test_subst_puts_runs_once(self):
        source = "subst {[puts hi]}\n"
        wasm, _ = _compile_tcl_with_diag(source)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        # Exactly one "hi" line — two-pass would print twice.
        assert stdout == "hi\n"

    def test_subst_interleaved_var_and_side_effect(self):
        """``subst "$x[set x longvalue]$x"`` — the read/write
        interleave.  With single-pass semantics, the first ``$x``
        sees the original value, ``[set x longvalue]`` both
        returns its new value AND propagates the side effect, so
        the second ``$x`` sees ``longvalue``.  The key property:
        the output buffer is sized correctly even though the
        second read yields a longer string than the first —
        which is exactly the overflow case the two-pass approach
        re-introduced.
        """
        source = 'set x a\nset r [subst "$x[set x longvalue]$x"]\nputs r=$r\n'
        wasm, _ = _compile_tcl_with_diag(source)
        _, stdout = _run_wasm(wasm, capture_stdout=True)
        assert stdout == "r=alongvaluelongvalue\n"


class TestFile:
    """Tcl ``file`` command subcommands backed by wasi-libc.

    Each test runs against a fresh host ``tmp_path`` that's
    preopened into the WASI instance as guest-path ``/``.  The
    compiled Tcl script calls ``file`` operations on paths
    rooted at ``/``; those resolve under ``tmp_path`` on the
    host.  Without the preopen, every ``access``/``stat`` call
    would return ``ENOTCAPABLE`` and these tests would assert
    the wrong thing.
    """

    def _run(self, tmp_path, tcl_source):
        """Compile + run *tcl_source* with *tmp_path* preopened
        as guest ``/``.  Returns captured stdout."""
        wasm, _ = _compile_tcl_with_diag(tcl_source, "t.tcl")
        _, stdout, _ = _run_wasm(
            wasm,
            capture_stdout=True,
            capture_stderr=True,
            preopen_tmpdir=str(tmp_path),
        )
        return stdout

    def test_exists_on_existing_file(self, tmp_path):
        (tmp_path / "hello.txt").write_text("hi")
        out = self._run(
            tmp_path,
            "puts [file exists /hello.txt]\n",
        )
        assert out.strip() == "1", f"got {out!r}"

    def test_exists_on_missing_file(self, tmp_path):
        out = self._run(
            tmp_path,
            "puts [file exists /does-not-exist]\n",
        )
        assert out.strip() == "0", f"got {out!r}"

    def test_isdirectory_vs_isfile(self, tmp_path):
        (tmp_path / "subdir").mkdir()
        (tmp_path / "file.txt").write_text("x")
        out = self._run(
            tmp_path,
            (
                "puts [file isdirectory /subdir]\n"
                "puts [file isdirectory /file.txt]\n"
                "puts [file isfile /subdir]\n"
                "puts [file isfile /file.txt]\n"
            ),
        )
        # isdirectory subdir=1, isdirectory file=0, isfile subdir=0, isfile file=1
        assert out.strip().splitlines() == ["1", "0", "0", "1"]

    def test_readable_writable(self, tmp_path):
        (tmp_path / "rw.txt").write_text("x")
        out = self._run(
            tmp_path,
            (
                "puts [file readable /rw.txt]\n"
                "puts [file writable /rw.txt]\n"
                "puts [file readable /nope]\n"
            ),
        )
        assert out.strip().splitlines() == ["1", "1", "0"]

    def test_size(self, tmp_path):
        (tmp_path / "data.bin").write_bytes(b"0123456789")
        out = self._run(
            tmp_path,
            "puts [file size /data.bin]\n",
        )
        assert out.strip() == "10"

    def test_mkdir_creates_directory(self, tmp_path):
        out = self._run(
            tmp_path,
            ("file mkdir /newdir\nputs [file isdirectory /newdir]\n"),
        )
        assert out.strip() == "1"
        assert (tmp_path / "newdir").is_dir()

    def test_mkdir_creates_nested_dirs(self, tmp_path):
        out = self._run(
            tmp_path,
            ("file mkdir /a/b/c\nputs [file isdirectory /a/b/c]\n"),
        )
        assert out.strip() == "1"
        assert (tmp_path / "a" / "b" / "c").is_dir()

    def test_mkdir_idempotent(self, tmp_path):
        (tmp_path / "existing").mkdir()
        # Second mkdir on an existing dir should be a no-op, not
        # an error.
        out = self._run(
            tmp_path,
            ("file mkdir /existing\nputs [file isdirectory /existing]\n"),
        )
        assert out.strip() == "1"

    def test_delete_file(self, tmp_path):
        (tmp_path / "gone.txt").write_text("x")
        out = self._run(
            tmp_path,
            ("file delete /gone.txt\nputs [file exists /gone.txt]\n"),
        )
        assert out.strip() == "0"
        assert not (tmp_path / "gone.txt").exists()

    def test_delete_empty_directory(self, tmp_path):
        (tmp_path / "emptydir").mkdir()
        out = self._run(
            tmp_path,
            ("file delete /emptydir\nputs [file exists /emptydir]\n"),
        )
        assert out.strip() == "0"

    def test_delete_missing_is_noop(self, tmp_path):
        # Tcl semantics: deleting a non-existent path succeeds
        # silently.
        out = self._run(
            tmp_path,
            ("file delete /never-existed\nputs done\n"),
        )
        assert "done" in out

    def test_rename(self, tmp_path):
        (tmp_path / "before.txt").write_text("hi")
        out = self._run(
            tmp_path,
            (
                "file rename /before.txt /after.txt\n"
                "puts [file exists /before.txt]\n"
                "puts [file exists /after.txt]\n"
            ),
        )
        assert out.strip().splitlines() == ["0", "1"]
        assert (tmp_path / "after.txt").read_text() == "hi"

    def test_copy(self, tmp_path):
        (tmp_path / "src.txt").write_text("payload")
        out = self._run(
            tmp_path,
            (
                "file copy /src.txt /dst.txt\n"
                "puts [file exists /src.txt]\n"
                "puts [file exists /dst.txt]\n"
                "puts [file size /dst.txt]\n"
            ),
        )
        assert out.strip().splitlines() == ["1", "1", "7"]
        assert (tmp_path / "dst.txt").read_text() == "payload"

    def test_type_file_vs_directory(self, tmp_path):
        (tmp_path / "f.txt").write_text("x")
        (tmp_path / "d").mkdir()
        out = self._run(
            tmp_path,
            "puts [file type /f.txt]\nputs [file type /d]\n",
        )
        assert out.strip().splitlines() == ["file", "directory"]

    def test_system(self, tmp_path):
        out = self._run(
            tmp_path,
            "puts [file system /]\n",
        )
        assert "native" in out

    def test_stat_returns_empty_and_sets_elements(self, tmp_path):
        # ``file stat`` returns empty and sets the array; we verify
        # the elements were created via ``info exists``.  Reading
        # back ``$st(size)`` hits a separate compiler-side gap for
        # interpolated array references in compiled code that's
        # tracked separately; once fixed, extend this test.
        (tmp_path / "stat-me.bin").write_bytes(b"1234567")
        out = self._run(
            tmp_path,
            (
                "puts [file stat /stat-me.bin st]\n"
                "puts [info exists st(size)]\n"
                "puts [info exists st(type)]\n"
                "puts [info exists st(mode)]\n"
            ),
        )
        # Splitlines rather than strip → preserves the leading
        # blank from ``file stat`` returning empty.
        lines = out.splitlines()
        # First line: file stat's return value (empty).  The rest
        # are the info-exists probes — every stat field present.
        assert lines == ["", "1", "1", "1"], f"got {lines!r}"

    def test_lstat_on_symlink(self, tmp_path):
        (tmp_path / "target.txt").write_text("hi")
        (tmp_path / "alias.txt").symlink_to("target.txt")
        # ``file lstat`` populates an array; verify the array
        # element was created.  ``file type`` via the direct
        # return path covers the "does it see link vs file"
        # distinction more directly — here we just ensure lstat
        # doesn't trap.
        out = self._run(
            tmp_path,
            (
                "puts [file lstat /alias.txt st]\n"
                "puts [file type /alias.txt]\n"
                "puts [file type /target.txt]\n"
            ),
        )
        lines = out.splitlines()
        assert lines == ["", "link", "file"], f"got {lines!r}"

    def test_readlink(self, tmp_path):
        (tmp_path / "real.txt").write_text("x")
        (tmp_path / "lnk.txt").symlink_to("real.txt")
        out = self._run(
            tmp_path,
            "puts [file readlink /lnk.txt]\n",
        )
        assert out.strip() == "real.txt"

    def test_link_creates_symlink(self, tmp_path):
        # WASI's ``path_symlink`` takes the target verbatim (it's
        # interpreted on read relative to the link's directory),
        # so we pass a relative target.
        (tmp_path / "tgt.txt").write_text("hello")
        out = self._run(
            tmp_path,
            ("file link /ln.txt tgt.txt\nputs [file type /ln.txt]\nputs [file readlink /ln.txt]\n"),
        )
        lines = out.splitlines()
        assert lines == ["link", "tgt.txt"], f"got {lines!r}"

    def test_file_link_returns_link_name(self, tmp_path):
        """Per Tcl semantics, ``file link linkName target`` returns
        ``linkName`` — not the target.  Regression test for the
        previous implementation that returned the target.
        """
        out = self._run(
            tmp_path,
            ("set r [file link /lnk tgt.txt]\nputs $r\n"),
        )
        assert out.strip() == "/lnk", f"got {out!r}"


class TestChannelIO:
    """End-to-end ``open`` / ``puts`` / ``gets`` / ``read`` / ``seek`` /
    ``tell`` / ``eof`` / ``close`` round-trips against a preopened
    tmpdir.  Same WASI capability model as :class:`TestFile`: the
    host ``tmp_path`` is mapped to guest ``/`` so paths like
    ``/log.txt`` resolve under it.
    """

    def _run(self, tmp_path, tcl_source):
        wasm, _ = _compile_tcl_with_diag(tcl_source, "t.tcl")
        _, stdout, _ = _run_wasm(
            wasm,
            capture_stdout=True,
            capture_stderr=True,
            preopen_tmpdir=str(tmp_path),
        )
        return stdout

    def test_puts_to_file_then_read_back(self, tmp_path):
        out = self._run(
            tmp_path,
            (
                "set fd [open /log.txt w]\n"
                'puts $fd "hello"\n'
                'puts $fd "world"\n'
                "close $fd\n"
                "set rfd [open /log.txt r]\n"
                "set body [read $rfd]\n"
                "close $rfd\n"
                "puts $body\n"
            ),
        )
        # The compiled-to-host newline format is LF on WASI.
        assert out == "hello\nworld\n\n", f"got {out!r}"
        # File on disk holds the same bytes.
        assert (tmp_path / "log.txt").read_text() == "hello\nworld\n"

    def test_gets_reads_lines(self, tmp_path):
        (tmp_path / "lines.txt").write_text("alpha\nbeta\ngamma\n")
        out = self._run(
            tmp_path,
            (
                "set fd [open /lines.txt r]\n"
                "puts [gets $fd]\n"
                "puts [gets $fd]\n"
                "puts [gets $fd]\n"
                "close $fd\n"
            ),
        )
        assert out == "alpha\nbeta\ngamma\n", f"got {out!r}"

    def test_gets_with_var_returns_length(self, tmp_path):
        (tmp_path / "lines.txt").write_text("hi\nworld\n")
        out = self._run(
            tmp_path,
            ("set fd [open /lines.txt r]\nset n [gets $fd line]\nputs $n\nputs $line\nclose $fd\n"),
        )
        assert out == "2\nhi\n", f"got {out!r}"

    def test_seek_and_tell_roundtrip(self, tmp_path):
        (tmp_path / "data.bin").write_bytes(b"0123456789")
        out = self._run(
            tmp_path,
            (
                "set fd [open /data.bin r]\n"
                "seek $fd 4\n"
                "puts [tell $fd]\n"
                "puts [read $fd 3]\n"
                "seek $fd 0 start\n"
                "puts [tell $fd]\n"
                "puts [read $fd 2]\n"
                "close $fd\n"
            ),
        )
        # Position after seek 4 = 4; read 3 chars = "456"; rewind to
        # 0; read 2 chars = "01".
        assert out == "4\n456\n0\n01\n", f"got {out!r}"

    def test_eof_after_full_read(self, tmp_path):
        (tmp_path / "f.txt").write_text("xyz")
        out = self._run(
            tmp_path,
            (
                "set fd [open /f.txt r]\n"
                "puts [eof $fd]\n"
                "puts [read $fd]\n"
                "puts [eof $fd]\n"
                "close $fd\n"
            ),
        )
        # eof is sticky once we've hit EOF on a read.
        lines = out.splitlines()
        assert lines[0] == "0"
        assert lines[1] == "xyz"
        assert lines[2] == "1"

    def test_fblocked_always_zero(self, tmp_path):
        (tmp_path / "f.txt").write_text("data")
        out = self._run(
            tmp_path,
            ("set fd [open /f.txt r]\nputs [fblocked $fd]\nclose $fd\n"),
        )
        assert out.strip() == "0"

    def test_append_mode_preserves_contents(self, tmp_path):
        (tmp_path / "log.txt").write_text("first\n")
        out = self._run(
            tmp_path,
            (
                "set fd [open /log.txt a]\n"
                'puts $fd "second"\n'
                "close $fd\n"
                "set fd [open /log.txt r]\n"
                "puts [read $fd]\n"
                "close $fd\n"
            ),
        )
        # Append leaves the original line and adds another.
        assert "first" in out and "second" in out, f"got {out!r}"
        assert (tmp_path / "log.txt").read_text() == "first\nsecond\n"

    def test_fcopy_between_channels(self, tmp_path):
        (tmp_path / "src.bin").write_bytes(b"copy-me-please")
        out = self._run(
            tmp_path,
            (
                "set in [open /src.bin r]\n"
                "set out [open /dst.bin w]\n"
                "set n [fcopy $in $out]\n"
                "close $in\n"
                "close $out\n"
                "puts $n\n"
            ),
        )
        assert out.strip() == "14"
        assert (tmp_path / "dst.bin").read_bytes() == b"copy-me-please"

    def test_open_missing_file_traps(self, tmp_path):
        wasm, _ = _compile_tcl_with_diag("set fd [open /no-such-file r]\n", "t.tcl")
        try:
            _run_wasm(
                wasm,
                capture_stdout=True,
                capture_stderr=True,
                preopen_tmpdir=str(tmp_path),
            )
            pytest.fail("expected trap")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "open" in stderr, f"stderr: {stderr!r}"

    def test_fcopy_after_partial_read_drains_buffer(self, tmp_path):
        """``fcopy`` must drain ``read``'s pull-side buffer first.

        Regression for codex P1: a prior ``read`` / ``gets`` prefetches
        bytes into the channel's pull buffer, which advances the OS
        file offset past the logical channel position.  Without
        consuming the buffered span, ``fcopy`` would skip those
        bytes (or return them out of order).
        """
        (tmp_path / "src.bin").write_bytes(b"abcdefghij")
        out = self._run(
            tmp_path,
            (
                "set in [open /src.bin r]\n"
                "set head [read $in 3]\n"  # consumes "abc", buffer now holds "defghij"
                "set out [open /dst.bin w]\n"
                "set n [fcopy $in $out]\n"
                "close $in\n"
                "close $out\n"
                "puts head=$head\n"
                "puts n=$n\n"
            ),
        )
        # fcopy should write the remaining 7 buffered/file bytes — not 0
        # (which is what an unaware fcopy would emit because read had
        # already pulled the whole 10-byte file into its 4 KB buffer).
        assert out.strip().splitlines() == ["head=abc", "n=7"], f"got {out!r}"
        assert (tmp_path / "dst.bin").read_bytes() == b"defghij"

    def test_read_rejects_non_integer_numchars(self, tmp_path):
        """Regression for codex P2: ``read $fd abc`` must error,
        not silently treat the bad arg as 'read to EOF'."""
        (tmp_path / "f.txt").write_text("xyz")
        wasm, _ = _compile_tcl_with_diag("set fd [open /f.txt r]\nread $fd abc\n", "t.tcl")
        try:
            _run_wasm(
                wasm,
                capture_stderr=True,
                preopen_tmpdir=str(tmp_path),
            )
            pytest.fail("expected trap on non-integer numChars")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "read" in stderr and "integer" in stderr, f"stderr: {stderr!r}"

    def test_read_rejects_negative_numchars(self, tmp_path):
        """Regression for codex P2: ``read $fd -2`` must error."""
        (tmp_path / "f.txt").write_text("xyz")
        wasm, _ = _compile_tcl_with_diag("set fd [open /f.txt r]\nread $fd -2\n", "t.tcl")
        try:
            _run_wasm(
                wasm,
                capture_stderr=True,
                preopen_tmpdir=str(tmp_path),
            )
            pytest.fail("expected trap on negative numChars")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "read" in stderr and "non-negative" in stderr, f"stderr: {stderr!r}"

    def test_fcopy_rejects_unknown_option(self, tmp_path):
        """Regression for codex P2: ``fcopy a b -foo 1`` must error."""
        (tmp_path / "src.bin").write_text("abc")
        wasm, _ = _compile_tcl_with_diag(
            ("set in [open /src.bin r]\nset out [open /dst.bin w]\nfcopy $in $out -foo 1\n"),
            "t.tcl",
        )
        try:
            _run_wasm(
                wasm,
                capture_stderr=True,
                preopen_tmpdir=str(tmp_path),
            )
            pytest.fail("expected trap on unknown option")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "fcopy" in stderr and "bad option" in stderr, f"stderr: {stderr!r}"

    def test_fcopy_rejects_non_integer_size(self, tmp_path):
        """Regression for codex P2: ``fcopy a b -size abc`` must error."""
        (tmp_path / "src.bin").write_text("abc")
        wasm, _ = _compile_tcl_with_diag(
            ("set in [open /src.bin r]\nset out [open /dst.bin w]\nfcopy $in $out -size abc\n"),
            "t.tcl",
        )
        try:
            _run_wasm(
                wasm,
                capture_stderr=True,
                preopen_tmpdir=str(tmp_path),
            )
            pytest.fail("expected trap on non-integer -size")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "fcopy" in stderr and "integer" in stderr, f"stderr: {stderr!r}"


class TestEncoding:
    """Minimal UTF-8 encoding command — pass-through for identity/utf-8."""

    @pytest.mark.parametrize(
        ("source", "expected_stdout"),
        [
            ("puts [encoding convertfrom identity abc]\n", "abc\n"),
            ("puts [encoding convertfrom utf-8 hello]\n", "hello\n"),
            ("puts [encoding convertto identity xyz]\n", "xyz\n"),
            ("puts [encoding convertto utf-8 data]\n", "data\n"),
            # Two-arg form defaults to system encoding (utf-8) and
            # passes through.
            ("puts [encoding convertfrom abc]\n", "abc\n"),
            ("puts [encoding system]\n", "utf-8\n"),
            ("puts [encoding names]\n", "identity utf-8\n"),
            ("puts [encoding dirs]\n", "\n"),
        ],
    )
    def test_passthrough(self, source, expected_stdout):
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        assert result[1] == expected_stdout

    @pytest.mark.parametrize(
        ("source", "expected_cmd"),
        [
            ("encoding convertfrom iso8859-1 abc\n", "encoding"),
            ("encoding convertfrom cp1252 abc\n", "encoding"),
            ("encoding convertto shiftjis xyz\n", "encoding"),
            ("encoding bogus_subcommand\n", "encoding"),
        ],
    )
    def test_unsupported_encoding_traps(self, source, expected_cmd):
        """Non-identity, non-utf-8 codecs must trap rather than pass through."""
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        try:
            _run_wasm(wasm, capture_stderr=True)
            pytest.fail("expected trap")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert f"unsupported command: {expected_cmd}" in stderr or (
                "encoding" in stderr and "unsupported" in stderr
            ), f"stderr: {stderr!r}"


class TestFconfigure:
    """fconfigure pass-through — accept benign options, trap on unknown ones."""

    @pytest.mark.parametrize(
        "source",
        [
            "fconfigure stdout -profile tcl8 -encoding utf-8\nputs ok\n",
            "fconfigure stdout -blocking 0\nputs ok\n",
            "fconfigure stdout -buffering line\nputs ok\n",
            "fconfigure stdout -translation binary\nputs ok\n",
            # Multiple pairs in one call.
            "fconfigure stdout -blocking 1 -buffering none -encoding utf-8\nputs ok\n",
        ],
    )
    def test_accepted_options(self, source):
        """Any option in the safelist may be set without trapping."""
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        assert result[1] == "ok\n"

    @pytest.mark.parametrize(
        "source",
        [
            "fconfigure stdout -nonsense value\nputs ok\n",
            "fconfigure stdout -polarity reverse\nputs ok\n",
            # Single ``-option`` queries for unknown options also trap.
            "fconfigure stdout -froboz\nputs ok\n",
        ],
    )
    def test_unknown_option_traps(self, source):
        """Options outside the safelist must raise unsupported."""
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        try:
            _run_wasm(wasm, capture_stderr=True)
            pytest.fail("expected trap")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "unsupported command: fconfigure" in stderr, f"stderr: {stderr!r}"


class TestFconfigureQuery:
    """fconfigure query forms — single-option and all-options.

    Covers the three shapes real Tcl supports:
      * ``fconfigure $fd``        → list of every -option/value pair.
      * ``fconfigure $fd -opt``   → that single option's value.
      * ``fconfigure $fd -o v …`` → setter (already covered above).

    Each set→get round-trip lands on the WASM channel struct in
    ``runtime/zig/io/tcl_chan.zig``; the renderers there map enum
    state back to canonical Tcl strings.
    """

    @pytest.mark.parametrize(
        "set_clause,opt,expected",
        [
            ("-translation lf", "-translation", "lf"),
            ("-translation crlf", "-translation", "crlf"),
            ("-buffering line", "-buffering", "line"),
            ("-buffering none", "-buffering", "none"),
            ("-buffersize 8192", "-buffersize", "8192"),
            ("-blocking 0", "-blocking", "0"),
            ("-blocking 1", "-blocking", "1"),
            ("-encoding utf-16", "-encoding", "utf-16"),
            ("-encoding binary", "-encoding", "binary"),
            ("-profile tcl8", "-profile", "tcl8"),
        ],
    )
    def test_set_get_round_trip(self, set_clause, opt, expected):
        # Force translation to lf before the puts so the captured
        # stdout doesn't pick up CRLF expansion when the test
        # exercises a translation other than lf.
        source = (
            f"fconfigure stdout {set_clause}\n"
            f"set v [fconfigure stdout {opt}]\n"
            "fconfigure stdout -translation lf\n"
            "puts $v\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        assert result[1] == f"{expected}\n"

    def test_query_all_options_dict_form(self):
        source = (
            "fconfigure stdout -translation lf -buffering line "
            "-buffersize 4096 -encoding utf-8 -profile tcl8\n"
            "puts [fconfigure stdout]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        out = result[1]
        # Every accepted option is present as ``-name value`` pair.
        for needle in (
            "-blocking 1",
            "-buffering line",
            "-buffersize 4096",
            "-encoding utf-8",
            "-eofchar {}",
            "-profile tcl8",
            "-translation lf",
        ):
            assert needle in out, f"missing {needle!r} in {out!r}"

    def test_dict_get_over_query_all(self):
        # Real Tcl idiom: ``dict get [fconfigure $fd] -encoding`` —
        # confirms the no-args query produces a well-formed dict.
        source = (
            "fconfigure stdout -encoding utf-16 -translation lf\n"
            "puts [dict get [fconfigure stdout] -encoding]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        assert result[1] == "utf-16\n"

    def test_eofchar_empty_round_trip(self):
        # ``fconfigure $fd -eofchar {}`` must reach the runtime as a
        # setter, not collapse to a bare ``-eofchar`` query.  The
        # producer-side list-quoting in the codegen literal path
        # and ``cmds/chan.zig:eval_fconfigure`` are responsible.
        source = (
            "fconfigure stdout -eofchar {} -translation lf\n"
            "puts [dict get [fconfigure stdout] -eofchar]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        # Empty eofchar renders as ``{}`` inside the all-options
        # list — that's the canonical Tcl list element for empty.
        assert result[1] == "\n"

    def test_eofchar_set_get_single_query(self):
        # The single-option query form returns the raw value (not
        # list-element-quoted).  An eofchar of ``Z`` should round-trip
        # as ``Z``, not ``{Z}``.
        source = "fconfigure stdout -eofchar Z -translation lf\nputs [fconfigure stdout -eofchar]\n"
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        assert result[1] == "Z\n"

    def test_encoding_with_space_round_trips_in_query_all(self):
        # Hypothetical encoding name with a space — the all-options
        # query must list-quote it so ``dict get`` recovers the
        # original string.  Exercises the ``list_elem_quote_nth``
        # path in ``render_option_value``.
        source = (
            "fconfigure stdout -encoding {utf 16} -translation lf\n"
            "puts [dict get [fconfigure stdout] -encoding]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        assert result[1] == "utf 16\n"

    def test_long_encoding_round_trips(self):
        # Storing options via TclObj refs (not a fixed-size buffer)
        # means long values round-trip without truncation.
        long_name = "x" * 64
        source = (
            f"fconfigure stdout -encoding {long_name} -translation lf\n"
            "puts [fconfigure stdout -encoding]\n"
        )
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        result = _run_wasm(wasm, capture_stdout=True)
        assert result[1] == long_name + "\n"

    def test_buffersize_invalid_raises(self):
        # Non-integer ``-buffersize`` must trap rather than silently
        # ignoring the option.
        source = "fconfigure stdout -buffersize abc\nputs ok\n"
        wasm, _ = _compile_tcl_with_diag(source, "t.tcl")
        try:
            _run_wasm(wasm, capture_stderr=True)
            pytest.fail("expected trap")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "expected integer value for -buffersize" in stderr, f"stderr: {stderr!r}"


class TestChannelTranslation:
    """Write-side translation + buffering edge cases.

    Backs `runtime/zig/io/tcl_chan.zig` against the upstream
    ``io-2.x`` scenarios (``WriteBytes: savedLF > 0`` and
    ``WriteBytes: flush on line``).  Each case writes through a
    real WASI fd into a host file under ``tmp_path`` and then
    re-reads the bytes from the host side to bypass the
    WASM-side translation that ``open ... r`` would apply.
    """

    def _run_then_read(self, tmp_path, tcl_source, target):
        wasm, _ = _compile_tcl_with_diag(tcl_source, "t.tcl")
        _run_wasm(
            wasm,
            capture_stdout=True,
            capture_stderr=True,
            preopen_tmpdir=str(tmp_path),
        )
        return (tmp_path / target).read_bytes()

    def test_crlf_translation_basic(self, tmp_path):
        """``-translation crlf`` expands LF → CRLF on flush/close."""
        bytes_out = self._run_then_read(
            tmp_path,
            (
                "set fd [open /out.txt w]\n"
                "fconfigure $fd -translation crlf\n"
                'puts -nonewline $fd "abc\ndef"\n'
                "close $fd\n"
            ),
            "out.txt",
        )
        assert bytes_out == b"abc\r\ndef", f"got {bytes_out!r}"

    def test_crlf_savedlf_boundary(self, tmp_path):
        """The CRLF expansion must split cleanly across a buffer boundary.

        Mirrors upstream ``io-2.2 {WriteBytes: savedLF > 0}``: the
        ``\\r`` lands at the end of the first 16-byte buffer and the
        ``\\n`` carries into the next refill.  Final bytes on disk
        must still be the full 19-byte CRLF sequence.
        """
        bytes_out = self._run_then_read(
            tmp_path,
            (
                "set fd [open /out.txt w]\n"
                "fconfigure $fd -translation crlf -buffersize 16\n"
                'puts -nonewline $fd "123456789012345\n12"\n'
                "close $fd\n"
            ),
            "out.txt",
        )
        assert bytes_out == b"123456789012345\r\n12", f"got {bytes_out!r}"

    def test_buffering_full_holds_until_close(self, tmp_path):
        """``-buffering full`` keeps bytes in memory until flush/close."""
        bytes_out = self._run_then_read(
            tmp_path,
            (
                "set fd [open /out.txt w]\n"
                "fconfigure $fd -translation lf -buffering full -buffersize 4096\n"
                'puts -nonewline $fd "hello"\n'
                # No flush here — close drains the buffer.
                "close $fd\n"
            ),
            "out.txt",
        )
        assert bytes_out == b"hello", f"got {bytes_out!r}"

    def test_buffering_line_flushes_on_newline(self, tmp_path):
        """``-buffering line`` drains after each ``\\n``."""
        # The script exits *without* closing the channel; the bytes
        # before the ``\n`` must still reach disk because line
        # buffering flushed them.  The ``\n`` itself flushes too;
        # bytes after it stay buffered and are lost on exit (no
        # atexit hook in the runtime), which is the difference
        # between full and line buffering we're checking.
        bytes_out = self._run_then_read(
            tmp_path,
            (
                "set fd [open /out.txt w]\n"
                "fconfigure $fd -translation lf -buffering line\n"
                'puts $fd "first"\n'
                'puts -nonewline $fd "tail"\n'
                # Intentionally no close — the "first\n" line must
                # have already landed on disk via the line flush.
            ),
            "out.txt",
        )
        assert bytes_out == b"first\n", f"got {bytes_out!r}"

    def test_buffering_none_flushes_immediately(self, tmp_path):
        """``-buffering none`` lands every ``puts`` straight on the fd."""
        bytes_out = self._run_then_read(
            tmp_path,
            (
                "set fd [open /out.txt w]\n"
                "fconfigure $fd -translation lf -buffering none\n"
                'puts -nonewline $fd "abc"\n'
                # No close — nothing should be left to flush; the
                # bytes already hit disk on the puts.
            ),
            "out.txt",
        )
        assert bytes_out == b"abc", f"got {bytes_out!r}"

    def test_explicit_flush_drains_buffer(self, tmp_path):
        """``flush $fd`` against a full-buffered channel drains it."""
        bytes_out = self._run_then_read(
            tmp_path,
            (
                "set fd [open /out.txt w]\n"
                "fconfigure $fd -translation lf -buffering full\n"
                'puts -nonewline $fd "queued"\n'
                "flush $fd\n"
                # Without close, the explicit flush is the only way
                # for those bytes to reach disk.
            ),
            "out.txt",
        )
        assert bytes_out == b"queued", f"got {bytes_out!r}"


class TestChan:
    """``chan <sub>`` ensemble — mirrors :class:`TestChannelIO` round-trips
    using the ``chan`` ensemble form rather than the bare-command form.
    Both forms route to the same handlers in ``cmds/io.zig`` /
    ``cmds/chan.zig``, so this class exercises the dispatch wiring in
    :func:`cmds/chan.zig:eval_chan`.
    """

    def _run(self, tmp_path, tcl_source):
        wasm, _ = _compile_tcl_with_diag(tcl_source, "t.tcl")
        _, stdout, _ = _run_wasm(
            wasm,
            capture_stdout=True,
            capture_stderr=True,
            preopen_tmpdir=str(tmp_path),
        )
        return stdout

    def test_chan_puts_then_chan_read(self, tmp_path):
        out = self._run(
            tmp_path,
            (
                "set fd [open /log.txt w]\n"
                'chan puts $fd "hello"\n'
                'chan puts $fd "world"\n'
                "chan close $fd\n"
                "set rfd [open /log.txt r]\n"
                "set body [chan read $rfd]\n"
                "chan close $rfd\n"
                "puts $body\n"
            ),
        )
        assert out == "hello\nworld\n\n", f"got {out!r}"
        assert (tmp_path / "log.txt").read_text() == "hello\nworld\n"

    def test_chan_gets_lines(self, tmp_path):
        (tmp_path / "lines.txt").write_text("alpha\nbeta\ngamma\n")
        out = self._run(
            tmp_path,
            (
                "set fd [open /lines.txt r]\n"
                "puts [chan gets $fd]\n"
                "puts [chan gets $fd]\n"
                "puts [chan gets $fd]\n"
                "chan close $fd\n"
            ),
        )
        assert out == "alpha\nbeta\ngamma\n", f"got {out!r}"

    def test_chan_gets_with_var(self, tmp_path):
        (tmp_path / "lines.txt").write_text("hi\nworld\n")
        out = self._run(
            tmp_path,
            (
                "set fd [open /lines.txt r]\n"
                "set n [chan gets $fd line]\n"
                "puts $n\n"
                "puts $line\n"
                "chan close $fd\n"
            ),
        )
        assert out == "2\nhi\n", f"got {out!r}"

    def test_chan_seek_and_tell(self, tmp_path):
        (tmp_path / "data.bin").write_bytes(b"0123456789")
        out = self._run(
            tmp_path,
            (
                "set fd [open /data.bin r]\n"
                "chan seek $fd 4\n"
                "puts [chan tell $fd]\n"
                "puts [chan read $fd 3]\n"
                "chan seek $fd 0 start\n"
                "puts [chan tell $fd]\n"
                "puts [chan read $fd 2]\n"
                "chan close $fd\n"
            ),
        )
        assert out == "4\n456\n0\n01\n", f"got {out!r}"

    def test_chan_eof_after_full_read(self, tmp_path):
        (tmp_path / "f.txt").write_text("xyz")
        out = self._run(
            tmp_path,
            (
                "set fd [open /f.txt r]\n"
                "puts [chan eof $fd]\n"
                "puts [chan read $fd]\n"
                "puts [chan eof $fd]\n"
                "chan close $fd\n"
            ),
        )
        lines = out.splitlines()
        assert lines[0] == "0"
        assert lines[1] == "xyz"
        assert lines[2] == "1"

    def test_chan_blocked_always_zero(self, tmp_path):
        (tmp_path / "f.txt").write_text("data")
        out = self._run(
            tmp_path,
            ("set fd [open /f.txt r]\nputs [chan blocked $fd]\nchan close $fd\n"),
        )
        assert out.strip() == "0"

    def test_chan_copy_between_channels(self, tmp_path):
        (tmp_path / "src.bin").write_bytes(b"copy-me-please")
        out = self._run(
            tmp_path,
            (
                "set in [open /src.bin r]\n"
                "set out [open /dst.bin w]\n"
                "set n [chan copy $in $out]\n"
                "chan close $in\n"
                "chan close $out\n"
                "puts $n\n"
            ),
        )
        assert out.strip() == "14"
        assert (tmp_path / "dst.bin").read_bytes() == b"copy-me-please"

    def test_chan_configure_translation(self, tmp_path):
        # ``chan configure $fd -translation auto`` is the call
        # ``chanio.test`` opens with — must not trap.
        out = self._run(
            tmp_path,
            (
                "set fd [open /log.txt w]\n"
                "chan configure $fd -translation auto\n"
                'chan puts -nonewline $fd "hi"\n'
                "chan close $fd\n"
                "puts ok\n"
            ),
        )
        assert out == "ok\n", f"got {out!r}"
        assert (tmp_path / "log.txt").read_text() == "hi"

    def test_chan_flush_is_noop(self, tmp_path):
        out = self._run(
            tmp_path,
            (
                "set fd [open /log.txt w]\n"
                'chan puts $fd "buffered"\n'
                "chan flush $fd\n"
                "chan close $fd\n"
                "puts ok\n"
            ),
        )
        assert out == "ok\n", f"got {out!r}"
        assert (tmp_path / "log.txt").read_text() == "buffered\n"

    def test_chan_names_includes_stdio(self, tmp_path):
        # Pre-populated slots are stdin / stdout / stderr.  ``chan
        # names`` must report at least those three when no user
        # channel is open.
        out = self._run(
            tmp_path,
            ("set names [chan names]\nputs [lsort $names]\n"),
        )
        names = out.strip().split()
        assert "stdin" in names, f"got {names!r}"
        assert "stdout" in names, f"got {names!r}"
        assert "stderr" in names, f"got {names!r}"

    def test_chan_names_includes_open_file(self, tmp_path):
        (tmp_path / "x.txt").write_text("x")
        out = self._run(
            tmp_path,
            ("set fd [open /x.txt r]\nputs $fd\nputs [chan names]\nchan close $fd\n"),
        )
        # The freshly-opened channel name should appear in
        # ``chan names`` alongside the standard streams.
        lines = out.splitlines()
        fd_name = lines[0]
        names = lines[1].split()
        assert fd_name in names, f"fd={fd_name!r} not in {names!r}"

    def test_chan_names_with_pattern(self, tmp_path):
        out = self._run(
            tmp_path,
            ("puts [lsort [chan names std*]]\n"),
        )
        # Glob filter narrows to the std* slots.
        assert out.strip().split() == ["stderr", "stdin", "stdout"]

    def test_chan_unknown_subcommand_traps(self, tmp_path):
        # Out-of-scope subcommand should trap with ``unsupported
        # command: chan``.
        wasm, _ = _compile_tcl_with_diag("chan create read foo\n", "t.tcl")
        try:
            _run_wasm(
                wasm,
                capture_stderr=True,
                preopen_tmpdir=str(tmp_path),
            )
            pytest.fail("expected trap on out-of-scope chan subcommand")
        except Exception as trap:
            stderr = getattr(trap, "tcl_stderr", "")
            assert "chan" in stderr and "unsupported" in stderr, f"stderr: {stderr!r}"


class TestExternalTcllibCounter:
    """Compile and run the real tcllib counter module (pure Tcl).

    https://github.com/tcltk/tcllib/tree/master/modules/counter
    14 procs, ~1200 lines of pure Tcl. Tests that a real-world tcllib
    module compiles to WASM and instantiates without trapping.

    ``upvar #0`` now produces a real global alias (see TestUpvarAndVariable);
    however, full counter semantics also require Tcl arrays
    (``set counter(N) 0``), which remain unimplemented. These tests
    verify compilation and dispatch end-to-end, not full counter behaviour.
    """

    _COUNTER_TCL = _EXTERNAL_DIR / "tcllib" / "counter" / "counter.tcl"

    def test_compiles(self):
        if not self._COUNTER_TCL.exists():
            pytest.skip(f"tcllib counter.tcl not present at {self._COUNTER_TCL}")
        source = self._COUNTER_TCL.read_text()
        wasm_bytes = _compile_tcl(source)
        # Should be a reasonable size
        assert len(wasm_bytes) > 5000
        assert len(wasm_bytes) < 100000

    def test_top_level_runs(self):
        """Running ::top should not trap (validates string table sharing)."""
        if not self._COUNTER_TCL.exists():
            pytest.skip(f"tcllib counter.tcl not present at {self._COUNTER_TCL}")
        source = self._COUNTER_TCL.read_text()
        wasm_bytes = _compile_tcl(source)
        val, _ = _run_wasm(wasm_bytes)
        assert val == 0

    def test_all_procs_exported(self):
        """All 14 counter procs should be exported."""
        if not self._COUNTER_TCL.exists():
            pytest.skip(f"tcllib counter.tcl not present at {self._COUNTER_TCL}")
        source = self._COUNTER_TCL.read_text()
        wasm_bytes = _compile_tcl(source)

        engine = _get_engine()
        module = wasmtime.Module(engine, wasm_bytes)
        export_names = {e.name for e in module.exports}
        expected_procs = {
            "::counter::init",
            "::counter::reset",
            "::counter::count",
            "::counter::exists",
            "::counter::get",
            "::counter::names",
            "::counter::start",
            "::counter::stop",
        }
        missing = expected_procs - export_names
        assert not missing, f"missing exports: {missing}"

    def test_init_proc_dispatches(self):
        """counter::init should dispatch without trapping."""
        if not self._COUNTER_TCL.exists():
            pytest.skip(f"tcllib counter.tcl not present at {self._COUNTER_TCL}")
        source = self._COUNTER_TCL.read_text()
        wasm_bytes = _compile_tcl(source)

        engine = _get_engine()
        store = wasmtime.Store(engine)
        wasi_config = wasmtime.WasiConfig()
        store.set_wasi(wasi_config)

        rt_mod = _get_rt_module()
        linker = wasmtime.Linker(engine)
        linker.define_wasi()
        tcl_instance_box, memory_box, rt_instance_box = _define_call_compiled_proc(linker, store)
        rt_inst = linker.instantiate(store, rt_mod)
        rt_instance_box[0] = rt_inst
        for export in rt_mod.exports:
            name = export.name
            if name.startswith("__"):
                continue
            val = rt_inst.exports(store)[name]
            if isinstance(val, wasmtime.Func):
                linker.define(store, "tcl", name, val)
            elif name == "memory":
                linker.define(store, "tcl", name, val)

        tcl_mod = wasmtime.Module(engine, wasm_bytes)
        tcl_inst = linker.instantiate(store, tcl_mod)
        tcl_instance_box[0] = tcl_inst
        memory_box[0] = rt_inst.exports(store)["memory"]

        exports = tcl_inst.exports(store)
        exports["::top"](store)  # Run top-level

        # Build a tag TclObj.  Grow the shared linear memory by one page
        # (64 KiB) and write the name bytes at the freshly-allocated region
        # so we don't collide with the runtime's heap, data segments, or
        # bump allocator.  memory.grow returns the previous page count, so
        # scratch_off is the start of the new pages in bytes.
        obj_new_string = rt_inst.exports(store)["obj_new_string"]
        memory = rt_inst.exports(store)["memory"]
        prev_pages = memory.grow(store, 1)
        scratch_off = prev_pages * 65536
        mem = memory.data_ptr(store)
        tag_bytes = b"simple"
        for i, b in enumerate(tag_bytes):
            mem[scratch_off + i] = b
        tag_obj = obj_new_string(store, scratch_off, len(tag_bytes))

        # counter::init {tag args} — 2 params.  ``args`` must be a real
        # (possibly empty) list TclObj — issue #263 made bare ``0``
        # (NULL TclObj) raise ``can't read "args": no such variable``
        # the moment the proc body reads ``$args``.  Pass an empty
        # string TclObj instead, which is the standard Tcl
        # representation of an empty ``args`` list.
        empty_args_obj = obj_new_string(store, scratch_off + len(tag_bytes), 0)
        result = exports["::counter::init"](store, tag_obj, empty_args_obj)
        # Should return a non-trapping result (actual value depends on semantics)
        assert isinstance(result, int)

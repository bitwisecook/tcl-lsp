"""WASM differential-fuzzer backend.

Compiles each fuzz script through the Tcl→WASM codegen, executes the
resulting module under wasmtime linked against the Zig value runtime,
and returns the same :class:`fuzzing.harness.RunResult` shape as the
Python VM and tclsh backends so codegen↔runtime divergences surface
in the same harness as VM bugs.

The runtime artefact is built on demand by
:func:`core.runtime_wasm.runtime_wasm_path` (no checked-in
``tcl_runtime.wasm``).  The wasmtime engine and runtime module are
loaded once per process and reused across every fuzz iteration; only
the per-script compiled module is rebuilt each call.

Stub filtering — :func:`uses_stubbed_command` flags scripts that hit a
``SILENT_STUB`` / ``TRAPPING_STUB`` command per
``tests/baselines/wasm_command_parity.json``.  Such scripts are
filtered at corpus-load time (see :mod:`fuzzing.runner`) rather than
treated as compare-time skips, so the WASM backend's results stay
honest for everything that does run through it.
"""

from __future__ import annotations

import json
import os
import tempfile
import threading
from functools import lru_cache
from pathlib import Path

from .harness import RunResult

_REPO_ROOT = Path(__file__).resolve().parent.parent
_PARITY_PATH = _REPO_ROOT / "tests" / "baselines" / "wasm_command_parity.json"

# Module-level caches — reused across fuzz iterations so the per-call
# cost is just compile-the-script + instantiate + run.
_engine = None  # type: ignore[var-annotated]
_rt_module = None  # type: ignore[var-annotated]
_rt_module_path: str | None = None

# Tracks how many times we've called ``increment_epoch`` on the cached
# engine.  ``set_epoch_deadline`` takes an *absolute* counter value, so
# every store must arm the deadline at ``current_count + 1`` — arming
# at a fixed ``1`` traps immediately as soon as a single prior watchdog
# has bumped the engine.  Mirrors the pattern in
# ``tests/test_wasm_real_tcl.py`` (which has the same constraint).
_epoch_count: int = 0


def _try_import_wasmtime():
    """Return the wasmtime module or None if unavailable."""
    try:
        import wasmtime  # noqa: PLC0415

        return wasmtime
    except ImportError:
        return None


def _get_engine():
    """Return a cached wasmtime engine with epoch interruption enabled.

    Epoch interruption lets a watchdog timer trap the WASM execution
    when a fuzz script runs away (infinite loop, expensive expr).  The
    same engine is reused across every fuzz iteration.
    """
    global _engine
    if _engine is None:
        wasmtime = _try_import_wasmtime()
        if wasmtime is None:
            return None
        cfg = wasmtime.Config()
        cfg.epoch_interruption = True
        _engine = wasmtime.Engine(cfg)
    return _engine


def _get_rt_module():
    """Return the cached compiled Zig runtime module.

    The runtime is built on demand on first access via
    ``core.runtime_wasm.runtime_wasm_path``.  The compiled module is
    bound to the engine returned by :func:`_get_engine`.
    """
    global _rt_module, _rt_module_path
    if _rt_module is not None:
        return _rt_module

    wasmtime = _try_import_wasmtime()
    if wasmtime is None:
        return None
    engine = _get_engine()
    if engine is None:
        return None

    from core.runtime_wasm import runtime_wasm_path  # noqa: PLC0415

    rt_path = runtime_wasm_path()
    if not rt_path.exists():
        return None
    _rt_module = wasmtime.Module.from_file(engine, str(rt_path))
    _rt_module_path = str(rt_path)
    return _rt_module


def is_available() -> bool:
    """Cheap check: can we run the WASM backend at all?

    Returns False if either ``wasmtime`` is not importable or the Zig
    runtime cannot be built/located.  Used by the runner to skip the
    backend gracefully on environments without the prerequisites.
    """
    if _try_import_wasmtime() is None:
        return False
    return _get_rt_module() is not None


@lru_cache(maxsize=1)
def _stubbed_commands() -> frozenset[str]:
    """Set of commands marked SILENT_STUB / TRAPPING_STUB in the parity baseline.

    Cached because the baseline doesn't change during a fuzz run.
    """
    if not _PARITY_PATH.exists():
        return frozenset()
    data = json.loads(_PARITY_PATH.read_text(encoding="utf-8"))
    statuses = data.get("command_status", {})
    stubbed = {
        name for name, status in statuses.items() if status in ("SILENT_STUB", "TRAPPING_STUB")
    }
    return frozenset(stubbed)


@lru_cache(maxsize=1)
def _stubbed_command_pattern():
    """Compiled regex matching a stubbed command at a command-start position.

    Tcl command boundaries are: start-of-line (``^``), newline,
    semicolon, ``{`` (start of a body / braced word), or ``[`` (start
    of a command substitution).  We require one of those — optionally
    followed by inline whitespace — before the command name, with a
    word boundary after.  This rules out:

    * variable reads / assignments — ``set switch 1`` (preceded by
      space, not a separator).
    * string literals — ``"the switch is on"`` (preceded by ``"``).
    * parameter / loop-variable names — ``foreach switch …``
      (preceded by space).
    * substring-of-identifier — ``my_switch_proc`` (no word break).

    …while still firing on commands inside braced bodies, since a
    body opens with ``{`` and the next non-whitespace token is the
    command word (the case the previous token-walk filter missed —
    the lexer treated braced bodies as a single ESC token).
    """
    import re  # noqa: PLC0415

    stubbed = _stubbed_commands()
    if not stubbed:
        # Pattern that never matches — keeps the API uniform.
        return re.compile(r"(?!x)x")
    # Sort longest-first so e.g. ``regex_quote`` is preferred over
    # any (currently nonexistent) prefix collision.
    alt = "|".join(re.escape(c) for c in sorted(stubbed, key=len, reverse=True))
    return re.compile(rf"(?:^|[\n;{{\[])\s*({alt})\b", re.MULTILINE)


def uses_stubbed_command(script: str) -> bool:
    """True if *script* invokes any WASM-stubbed command.

    Matches command names at Tcl command-start positions only — this
    is intentionally narrower than a substring search so non-command
    uses (``set switch 1``, ``foreach switch …``, ``"the switch
    is on"``) aren't falsely flagged and dropped from the corpus.
    See :func:`_stubbed_command_pattern` for the boundary conditions.
    """
    return bool(_stubbed_command_pattern().search(script))


def _compile_script(script: str) -> bytes:
    """Compile *script* to WASM bytes.

    Uses the same compile path as ``tests/test_wasm_execution.py`` and
    ``tests/test_wasm_real_tcl.py`` — IR lowering, CFG build, codegen.
    Compile errors propagate as exceptions for the caller to classify.
    """
    from core.compiler.cfg import build_cfg  # noqa: PLC0415
    from core.compiler.codegen.wasm import wasm_codegen_module  # noqa: PLC0415
    from core.compiler.lowering import lower_to_ir  # noqa: PLC0415

    ir_module = lower_to_ir(script)
    cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=False)
    return wasm_module.to_bytes()


def _run_wasm_inner(
    wasm_bytes: bytes,
    *,
    timeout: float,
) -> RunResult:
    """Instantiate *wasm_bytes*, run ``::top``, and capture stdout/stderr.

    Errors are mapped onto the harness ``RunResult`` shape:

    * Tcl trap (``tcl trap: site=…`` in stderr) → ``return_code=1``.
    * Other wasmtime traps (memory unreachable, indirect-call type,
      etc.) → ``return_code=2`` since they indicate a runtime/codegen
      bug rather than a Tcl-level error.
    * Watchdog-driven epoch trap → ``return_code=2`` with
      ``error_message='TIMEOUT'`` to match the VM/tclsh contract.
    """
    wasmtime = _try_import_wasmtime()
    assert wasmtime is not None  # caller checked is_available()

    engine = _get_engine()
    rt_module = _get_rt_module()
    assert engine is not None and rt_module is not None

    store = wasmtime.Store(engine)
    # Watchdog: bump the engine's epoch after `timeout` seconds.  The
    # next wasm op then traps with an epoch-deadline error.
    #
    # ``set_epoch_deadline`` takes an *absolute* counter value.  The
    # engine is process-global, so ``_epoch_count`` mirrors how many
    # ``increment_epoch`` calls prior watchdog fires have made; the
    # next deadline must be strictly ahead of that or the store traps
    # immediately on first wasm op.  Arming at a fixed ``1`` (the
    # earlier shape of this code) was wrong — once any prior
    # iteration timed out, every subsequent run would trap before
    # ``::top`` started executing, poisoning the whole campaign.
    global _epoch_count
    deadline_value = _epoch_count + 1
    store.set_epoch_deadline(deadline_value)

    wasi_config = wasmtime.WasiConfig()
    fd_out, stdout_path = tempfile.mkstemp(suffix=".out")
    os.close(fd_out)
    fd_err, stderr_path = tempfile.mkstemp(suffix=".err")
    os.close(fd_err)
    wasi_config.stdout_file = stdout_path
    wasi_config.stderr_file = stderr_path
    store.set_wasi(wasi_config)

    linker = wasmtime.Linker(engine)
    linker.define_wasi()

    # Cross-context dispatch bridge — needed even if the script never
    # calls a compiled proc, because the runtime imports it.
    tcl_instance_box: list = [None]
    memory_box: list = [None]

    def _call_compiled_proc(name_ptr: int, name_len: int, argv_ptr: int, argc: int) -> int:
        inst = tcl_instance_box[0]
        mem = memory_box[0]
        if inst is None or mem is None:
            raise RuntimeError("call_compiled_proc invoked before wiring complete")
        raw = bytes(mem.data_ptr(store)[name_ptr : name_ptr + name_len])
        pname = raw.decode("utf-8", errors="replace")
        func = inst.exports(store).get(pname)
        if func is None:
            raise RuntimeError(f"call_compiled_proc: missing export {pname!r}")
        args: list[int] = []
        for i in range(argc):
            off = argv_ptr + i * 4
            b = bytes(mem.data_ptr(store)[off : off + 4])
            args.append(int.from_bytes(b, "little", signed=False))
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

    # ``env.host_spawn`` is imported by the runtime for capability-gated
    # ``exec`` (5180cdf).  The fuzzer never enables CAP_EXEC, so the
    # capability check rejects every call before dispatch — but the
    # import itself still has to be satisfied or instantiation traps
    # with ``unknown import``.  Trap loudly on accidental invocation
    # so a future harness change that flips CAP_EXEC on doesn't
    # silently look like every ``exec`` succeeded with empty output:
    # the runtime treats the i32 result as a TclObj string handle and
    # ``0`` (NULL) is indistinguishable from a captured-empty success.
    # Same posture as ``tests/runtime/_host_imports.py``'s default.
    def _host_spawn(_argv_ptr: int, _argv_len: int, _stdin_ptr: int, _stdin_len: int) -> int:
        raise RuntimeError(
            "fuzzing.wasm_backend: env.host_spawn invoked but CAP_EXEC "
            "is not enabled in the fuzzer harness — implement a real "
            "callback if exec coverage is intentional"
        )

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
        _host_spawn,
    )

    rt_instance = linker.instantiate(store, rt_module)

    # WASI reactor init — populates preopen FD table; no-op on older runtime
    init_fn = rt_instance.exports(store).get("_initialize")
    if init_fn is not None:
        init_fn(store)

    # Re-export runtime under "tcl" namespace for the compiled module's imports
    for export in rt_module.exports:
        name = export.name
        if name.startswith("__"):
            continue
        val = rt_instance.exports(store)[name]
        if isinstance(val, wasmtime.Func):
            linker.define(store, "tcl", name, val)
        elif name == "memory":
            linker.define(store, "tcl", name, val)

    tcl_module = wasmtime.Module(engine, wasm_bytes)
    tcl_instance = linker.instantiate(store, tcl_module)
    tcl_instance_box[0] = tcl_instance
    memory_box[0] = rt_instance.exports(store)["memory"]

    top_func = tcl_instance.exports(store).get("::top")
    if top_func is None:
        _read_and_unlink(stdout_path)
        _read_and_unlink(stderr_path)
        return RunResult(
            stdout="",
            stderr="",
            return_code=1,
            error_message="WASM_NO_TOP_EXPORT",
        )

    watchdog_fired = [False]

    def _bump_epoch():
        engine.increment_epoch()
        watchdog_fired[0] = True

    watchdog = threading.Timer(timeout, _bump_epoch)
    watchdog.daemon = True
    watchdog.start()

    try:
        top_func(store)
        stdout_text = _read_and_unlink(stdout_path)
        stderr_text = _read_and_unlink(stderr_path)
        return RunResult(stdout=stdout_text, stderr=stderr_text, return_code=0)
    except BaseException as exc:
        stdout_text = _read_and_unlink(stdout_path)
        stderr_text = _read_and_unlink(stderr_path)
        if watchdog_fired[0]:
            return RunResult(
                stdout=stdout_text,
                stderr=stderr_text,
                return_code=2,
                error_message="TIMEOUT",
            )
        # Tcl-level traps emit ``tcl trap: site=<id> <msg>`` to stderr
        # before unwinding.  Treat those as a normal Tcl error so the
        # harness compares them to vm/tclsh error paths.  Anything
        # else is a real runtime/codegen bug.
        if "tcl trap:" in stderr_text:
            msg = _extract_tcl_trap(stderr_text)
            return RunResult(
                stdout=stdout_text,
                stderr=stderr_text,
                return_code=1,
                error_message=msg,
            )
        return RunResult(
            stdout=stdout_text,
            stderr=stderr_text,
            return_code=2,
            error_message=f"WASM_TRAP: {type(exc).__name__}: {str(exc)[:300]}",
        )
    finally:
        watchdog.cancel()
        # Mirror the engine's epoch counter when the watchdog actually
        # fired (``increment_epoch`` was called).  A clean wasm return
        # leaves the counter where it was, so we leave ``_epoch_count``
        # alone in that case — bumping unconditionally would manufacture
        # ticks the engine never observed and eventually push deadlines
        # beyond what a single ``increment_epoch`` can reach.
        if watchdog_fired[0]:
            _epoch_count = max(_epoch_count, deadline_value)


def _extract_tcl_trap(stderr_text: str) -> str:
    """Pull the human-readable message out of a ``tcl trap:`` line."""
    import re  # noqa: PLC0415

    m = re.search(r"tcl trap:\s*(?:site=\d+\s*)?(.*)", stderr_text)
    if m:
        return m.group(1).strip()[:300] or "(unknown trap)"
    return stderr_text.strip()[:300] or "(unknown trap)"


def _read_and_unlink(path: str) -> str:
    """Read the WASI capture file and unlink it; tolerant of binary noise."""
    try:
        return Path(path).read_bytes().decode("utf-8", errors="replace")
    except FileNotFoundError:
        return ""
    finally:
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass


def run_wasm(script: str, *, timeout: float = 5.0) -> RunResult:
    """Compile *script* to WASM and execute it under wasmtime.

    Returns the same :class:`RunResult` shape as the other backends:

    * ``return_code == 0`` — top-level script ran cleanly.
    * ``return_code == 1`` — compile error, ``::top`` missing, or
      Tcl-level trap (i.e. an error the script itself caused).
    * ``return_code == 2`` — wasmtime trap with no ``tcl trap:`` line
      (codegen/runtime bug) or a timeout.
    """
    if not is_available():
        return RunResult(
            stdout="",
            stderr="",
            return_code=2,
            error_message="WASM_UNAVAILABLE",
        )

    try:
        wasm_bytes = _compile_script(script)
    except SystemExit:
        raise
    except KeyboardInterrupt:
        raise
    except Exception as exc:  # noqa: BLE001
        return RunResult(
            stdout="",
            stderr="",
            return_code=1,
            error_message=f"WASM_COMPILE_ERROR: {type(exc).__name__}: {str(exc)[:300]}",
        )

    try:
        return _run_wasm_inner(wasm_bytes, timeout=timeout)
    except SystemExit:
        raise
    except KeyboardInterrupt:
        raise
    except Exception as exc:  # noqa: BLE001
        return RunResult(
            stdout="",
            stderr="",
            return_code=2,
            error_message=f"WASM_HOST_ERROR: {type(exc).__name__}: {str(exc)[:300]}",
        )

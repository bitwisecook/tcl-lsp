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


def uses_stubbed_command(script: str) -> bool:
    """True if *script* invokes any WASM-stubbed command.

    Tokenises with the project lexer and inspects each command-start
    token.  Substring matching would false-positive on string literals
    like ``"the switch is on"``.  When the lexer fails we fall back to
    a conservative whole-word regex search — better to over-skip a few
    inputs than to feed a stubbed command into the WASM backend and
    record a phantom divergence.
    """
    stubbed = _stubbed_commands()
    if not stubbed:
        return False

    try:
        from core.parsing.lexer import TclLexer  # noqa: PLC0415

        lexer = TclLexer(script)
        tokens = lexer.tokenise_all()
    except Exception:
        # Fallback: whole-word match
        import re  # noqa: PLC0415

        for cmd in stubbed:
            if re.search(rf"(?:^|[\s;\[]){re.escape(cmd)}\b", script):
                return True
        return False

    # Token kinds vary across versions of the lexer; be tolerant.  We
    # look for ``Word``/``Bareword``-shaped tokens immediately after a
    # statement separator.  Any token whose text matches a stub name
    # forces the script to be filtered.
    for tok in tokens:
        text = getattr(tok, "text", None) or getattr(tok, "value", None)
        if text in stubbed:
            return True
    return False


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
    store.set_epoch_deadline(1)

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

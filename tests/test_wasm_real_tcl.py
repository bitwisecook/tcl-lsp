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
from core.compiler.codegen.wasm import wasm_codegen_module
from core.compiler.lowering import lower_to_ir

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

_ZIG_RUNTIME_PATH = (
    Path(__file__).resolve().parent.parent
    / "runtime"
    / "zig"
    / "zig-out"
    / "bin"
    / "tcl_runtime.wasm"
)

_SNIPPETS_DIR = Path(__file__).resolve().parent / "bytecode_snippets"
_FIXTURES_DIR = Path(__file__).resolve().parent / "fixtures"
_EXTERNAL_DIR = Path(__file__).resolve().parent / "external"

_engine: wasmtime.Engine | None = None
_rt_module: wasmtime.Module | None = None


def _get_engine() -> wasmtime.Engine:
    global _engine
    if _engine is None:
        _engine = wasmtime.Engine()
    return _engine


def _get_rt_module() -> wasmtime.Module:
    global _rt_module
    if _rt_module is None:
        if not _ZIG_RUNTIME_PATH.exists():
            pytest.skip(f"Zig WASM runtime not built: {_ZIG_RUNTIME_PATH}")
        _rt_module = wasmtime.Module.from_file(_get_engine(), str(_ZIG_RUNTIME_PATH))
    return _rt_module


def _compile_tcl(source: str) -> bytes:
    """Compile Tcl source to WASM bytes."""
    ir_module = lower_to_ir(source)
    cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=False)
    return wasm_module.to_bytes()


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
) -> tuple[int, str]:
    """Link and run a compiled Tcl WASM module.

    Returns (return_value, stdout_text).
    """
    engine = _get_engine()
    store = wasmtime.Store(engine)
    wasi_config = wasmtime.WasiConfig()

    stdout_path = None
    if capture_stdout:
        fd, stdout_path = tempfile.mkstemp(suffix=".txt")
        os.close(fd)
        wasi_config.stdout_file = stdout_path

    store.set_wasi(wasi_config)

    # Instantiate Zig runtime
    rt_module = _get_rt_module()
    linker = wasmtime.Linker(engine)
    linker.define_wasi()
    rt_instance = linker.instantiate(store, rt_module)

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

    obj_new_int = rt_instance.exports(store)["obj_new_int"]
    obj_get_int = rt_instance.exports(store)["obj_get_int"]

    func = tcl_instance.exports(store).get(func_name)
    if func is None:
        raise RuntimeError(f"function {func_name} not found in WASM exports")

    boxed_args = tuple(obj_new_int(store, a) for a in args)
    result_obj = func(store, *boxed_args)
    result_val = obj_get_int(store, result_obj) if result_obj else 0

    stdout_text = ""
    if stdout_path:
        try:
            stdout_text = Path(stdout_path).read_text()
        finally:
            os.unlink(stdout_path)

    return result_val, stdout_text


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
        rt_inst = linker.instantiate(store, rt_mod)
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

        # counter::init {tag args} — 2 params
        result = exports["::counter::init"](store, tag_obj, 0)
        # Should return a non-trapping result (actual value depends on semantics)
        assert isinstance(result, int)

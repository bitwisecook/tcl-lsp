"""Tests for the whole-program WASM linker.

Verifies that multiple Tcl sources can be merged and compiled into
a single WASM module, and that ``source`` commands are resolved at
compile time.
"""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from core.compiler.codegen.wasm_link import (
    merge_ir_modules,
    wasm_link,
    wasm_link_sources,
)
from core.compiler.lowering import lower_to_ir
from tests.test_wasm_execution import (  # noqa: E402
    _get_engine,
    _link_and_instantiate,
)

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")


def _run_linked_module(
    wasm_mod,
    func_name: str = "::top",
    args: tuple[int, ...] = (),
) -> int:
    """Run a function from a linked WASM module via the Zig runtime."""
    wasm_bytes = wasm_mod.to_bytes()
    engine = _get_engine()
    store = wasmtime.Store(engine)
    store.set_wasi(wasmtime.WasiConfig())
    tcl_inst, rt_inst = _link_and_instantiate(store, wasm_bytes)

    obj_new_int = rt_inst.exports(store)["obj_new_int"]
    obj_get_int = rt_inst.exports(store)["obj_get_int"]

    boxed = tuple(obj_new_int(store, a) for a in args)
    func = tcl_inst.exports(store)[func_name]
    result = func(store, *boxed)
    if result == 0:
        return 0
    return obj_get_int(store, result)


# IR merge


class TestMergeIrModules:
    """Test IR module merging."""

    def test_merge_two_modules(self):
        """Procs from both modules should appear in merged output."""
        ir_a = lower_to_ir("proc add {a b} { expr {$a + $b} }\n")
        ir_b = lower_to_ir("proc mul {a b} { expr {$a * $b} }\n")
        merged = merge_ir_modules(ir_a, ir_b)
        assert "::add" in merged.procedures
        assert "::mul" in merged.procedures

    def test_merge_top_level_order(self):
        """Top-level statements should be concatenated in order."""
        ir_a = lower_to_ir("set x 1\n")
        ir_b = lower_to_ir("set y 2\n")
        merged = merge_ir_modules(ir_a, ir_b)
        assert len(merged.top_level.statements) >= 2

    def test_merge_proc_override(self):
        """Later modules should override procs from earlier ones."""
        ir_a = lower_to_ir("proc f {x} { expr {$x + 1} }\n")
        ir_b = lower_to_ir("proc f {x} { expr {$x + 2} }\n")
        merged = merge_ir_modules(ir_a, ir_b)
        assert "::f" in merged.procedures
        assert "::f" in merged.redefined_procedures

    def test_merge_empty(self):
        """Merging with empty modules should not fail."""
        from core.compiler.ir import IRModule

        merged = merge_ir_modules(IRModule(), IRModule())
        assert len(merged.top_level.statements) == 0
        assert len(merged.procedures) == 0


# Multi-source compilation


class TestWasmLinkSources:
    """Test compiling multiple sources into a single WASM module."""

    def test_two_source_procs(self):
        """Procs from two sources should be callable in the merged module."""
        mod = wasm_link_sources(
            [
                ("lib.tcl", "proc double {x} { expr {$x * 2} }\n"),
                ("main.tcl", "proc caller {n} { double $n }\n"),
            ]
        )
        result = _run_linked_module(mod, "::caller", (7,))
        assert result == 14

    def test_cross_file_proc_call(self):
        """A proc in one file should be able to call a proc from another."""
        mod = wasm_link_sources(
            [
                (
                    "math.tcl",
                    "proc add {a b} { expr {$a + $b} }\nproc sub {a b} { expr {$a - $b} }\n",
                ),
                ("app.tcl", "proc calc {x y} { add [sub $x $y] $y }\n"),
            ]
        )
        # calc(10, 3) = add(sub(10,3), 3) = add(7, 3) = 10
        result = _run_linked_module(mod, "::calc", (10, 3))
        assert result == 10

    def test_three_files(self):
        """Three files merged into one module."""
        mod = wasm_link_sources(
            [
                ("a.tcl", "proc inc {x} { expr {$x + 1} }\n"),
                ("b.tcl", "proc dec {x} { expr {$x - 1} }\n"),
                ("c.tcl", "proc roundtrip {x} { dec [inc $x] }\n"),
            ]
        )
        result = _run_linked_module(mod, "::roundtrip", (42,))
        assert result == 42

    def test_optimised_matches_unoptimised(self):
        """Optimised and non-optimised merged modules should agree."""
        sources = [
            ("lib.tcl", "proc sq {x} { expr {$x * $x} }\n"),
            ("main.tcl", "proc f {n} { sq $n }\n"),
        ]
        mod_no = wasm_link_sources(sources, optimise=False)
        mod_opt = wasm_link_sources(sources, optimise=True)
        for n in (0, 3, 5, -2):
            r_no = _run_linked_module(mod_no, "::f", (n,))
            r_opt = _run_linked_module(mod_opt, "::f", (n,))
            assert r_no == r_opt == n * n, f"n={n}: no_opt={r_no}, opt={r_opt}"


# File-based source resolution


class TestWasmLink:
    """Test file-based whole-program linking with source resolution."""

    def test_single_file(self):
        """A single file with no source commands should compile."""
        with tempfile.TemporaryDirectory() as tmp:
            main = Path(tmp) / "main.tcl"
            main.write_text("proc add {a b} { expr {$a + $b} }\n")
            mod = wasm_link(main)
            result = _run_linked_module(mod, "::add", (3, 4))
            assert result == 7

    def test_source_resolution(self):
        """source commands should be resolved and bundled."""
        with tempfile.TemporaryDirectory() as tmp:
            lib = Path(tmp) / "lib.tcl"
            lib.write_text("proc double {x} { expr {$x * 2} }\n")
            main = Path(tmp) / "main.tcl"
            main.write_text("source lib.tcl\nproc caller {n} { double $n }\n")
            mod = wasm_link(main)
            result = _run_linked_module(mod, "::caller", (7,))
            assert result == 14

    def test_source_with_search_path(self):
        """source commands should be resolved via search_paths."""
        with tempfile.TemporaryDirectory() as tmp:
            libdir = Path(tmp) / "lib"
            libdir.mkdir()
            (libdir / "utils.tcl").write_text("proc square {x} { expr {$x * $x} }\n")
            main = Path(tmp) / "main.tcl"
            main.write_text("source utils.tcl\nproc f {n} { square $n }\n")
            mod = wasm_link(main, search_paths=(str(libdir),))
            result = _run_linked_module(mod, "::f", (5,))
            assert result == 25

    def test_transitive_source(self):
        """source chains should be resolved transitively."""
        with tempfile.TemporaryDirectory() as tmp:
            a = Path(tmp) / "a.tcl"
            a.write_text("proc inc {x} { expr {$x + 1} }\n")
            b = Path(tmp) / "b.tcl"
            b.write_text("source a.tcl\nproc double_inc {x} { inc [inc $x] }\n")
            main = Path(tmp) / "main.tcl"
            main.write_text("source b.tcl\nproc f {n} { double_inc $n }\n")
            mod = wasm_link(main)
            result = _run_linked_module(mod, "::f", (5,))
            assert result == 7

    def test_cycle_protection(self):
        """Circular source dependencies should not loop forever."""
        with tempfile.TemporaryDirectory() as tmp:
            a = Path(tmp) / "a.tcl"
            b = Path(tmp) / "b.tcl"
            a.write_text("source b.tcl\nproc fa {x} { return $x }\n")
            b.write_text("source a.tcl\nproc fb {x} { return $x }\n")
            # Should not hang — cycle detection prevents infinite recursion
            mod = wasm_link(a)
            assert "::fa" in [f.name for f in mod.functions]
            assert "::fb" in [f.name for f in mod.functions]

    def test_missing_source_ignored(self):
        """source commands for missing files should be silently ignored."""
        with tempfile.TemporaryDirectory() as tmp:
            main = Path(tmp) / "main.tcl"
            main.write_text("source nonexistent.tcl\nproc f {x} { return $x }\n")
            mod = wasm_link(main)
            result = _run_linked_module(mod, "::f", (42,))
            assert result == 42


# Package require resolution


class TestPackageRequire:
    """Test package require resolution in the linker."""

    def test_package_require_direct(self):
        """package require resolves to <name>/<name>.tcl."""
        with tempfile.TemporaryDirectory() as tmp:
            pkg_dir = Path(tmp) / "mylib"
            pkg_dir.mkdir()
            (pkg_dir / "mylib.tcl").write_text("proc mylib_add {a b} { expr {$a + $b} }\n")
            main = Path(tmp) / "main.tcl"
            main.write_text("package require mylib\nproc f {x y} { mylib_add $x $y }\n")
            mod = wasm_link(main, search_paths=(str(tmp),))
            result = _run_linked_module(mod, "::f", (10, 20))
            assert result == 30

    def test_package_require_pkg_index(self):
        """package require resolves via pkgIndex.tcl source commands."""
        with tempfile.TemporaryDirectory() as tmp:
            pkg_dir = Path(tmp) / "mathlib"
            pkg_dir.mkdir()
            (pkg_dir / "math_impl.tcl").write_text("proc math_mul {a b} { expr {$a * $b} }\n")
            (pkg_dir / "pkgIndex.tcl").write_text(
                "package ifneeded mathlib 1.0 [list source [file join $dir math_impl.tcl]]\n"
            )
            main = Path(tmp) / "main.tcl"
            main.write_text("package require mathlib\nproc f {x y} { math_mul $x $y }\n")
            mod = wasm_link(main, search_paths=(str(tmp),))
            result = _run_linked_module(mod, "::f", (6, 7))
            assert result == 42

    def test_package_require_tcl_module(self):
        """package require resolves Tcl Modules (<name>-<version>.tm)."""
        with tempfile.TemporaryDirectory() as tmp:
            tm_file = Path(tmp) / "helper-1.0.tm"
            tm_file.write_text("proc helper_inc {x} { expr {$x + 1} }\n")
            main = Path(tmp) / "main.tcl"
            main.write_text("package require helper\nproc f {n} { helper_inc $n }\n")
            mod = wasm_link(main, search_paths=(str(tmp),))
            result = _run_linked_module(mod, "::f", (9,))
            assert result == 10

    def test_package_require_missing_ignored(self):
        """package require for missing packages should not crash."""
        with tempfile.TemporaryDirectory() as tmp:
            main = Path(tmp) / "main.tcl"
            main.write_text("package require nonexistent\nproc f {x} { return $x }\n")
            mod = wasm_link(main, search_paths=(str(tmp),))
            result = _run_linked_module(mod, "::f", (42,))
            assert result == 42


# Integration: multi-file Tcl project


class TestIntegrationProject:
    """Compile and execute a realistic multi-file Tcl project.

    This exercises the linker, package resolution, and runtime together.
    """

    def test_multi_file_with_packages(self):
        """Whole-program compilation of a multi-file project with packages."""
        with tempfile.TemporaryDirectory() as tmp:
            # lib/mathutils/mathutils.tcl — arithmetic utilities
            mathdir = Path(tmp) / "lib" / "mathutils"
            mathdir.mkdir(parents=True)
            (mathdir / "mathutils.tcl").write_text(
                """\
proc math_abs {x} {
    if {$x < 0} { return [expr {-$x}] }
    return $x
}
proc math_max {a b} {
    if {$a > $b} { return $a }
    return $b
}
proc math_min {a b} {
    if {$a < $b} { return $a }
    return $b
}
proc math_clamp {x lo hi} {
    math_max [math_min $x $hi] $lo
}
"""
            )
            # lib/strutils-1.0.tm — string utilities (Tcl module)
            (Path(tmp) / "lib" / "strutils-1.0.tm").write_text(
                """\
proc str_is_empty {s} {
    expr {[string length $s] == 0}
}
"""
            )
            # helpers.tcl — sourced directly
            (Path(tmp) / "helpers.tcl").write_text(
                """\
proc sum_range {n} {
    set s 0
    for {set i 1} {$i <= $n} {incr i} {
        set s [expr {$s + $i}]
    }
    return $s
}
proc factorial {n} {
    if {$n <= 1} { return 1 }
    expr {$n * [factorial [expr {$n - 1}]]}
}
"""
            )
            # main.tcl — entry point
            (Path(tmp) / "main.tcl").write_text(
                """\
source helpers.tcl
package require mathutils
proc compute {n} {
    set s [sum_range $n]
    set clamped [math_clamp $s 0 100]
    return $clamped
}
"""
            )
            lib_dir = str(Path(tmp) / "lib")
            mod = wasm_link(Path(tmp) / "main.tcl", search_paths=(lib_dir,))
            # sum_range(10) = 55, clamp(55, 0, 100) = 55
            result = _run_linked_module(mod, "::compute", (10,))
            assert result == 55
            # sum_range(15) = 120, clamp(120, 0, 100) = 100
            result = _run_linked_module(mod, "::compute", (15,))
            assert result == 100

    def test_recursive_fibonacci_linked(self):
        """Recursive fibonacci across linked source files."""
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "fib.tcl").write_text(
                """\
proc fib {n} {
    if {$n <= 1} { return $n }
    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}
}
"""
            )
            (Path(tmp) / "main.tcl").write_text("source fib.tcl\n")
            mod = wasm_link(Path(tmp) / "main.tcl")
            assert _run_linked_module(mod, "::fib", (8,)) == 21

    def test_string_processing_pipeline(self):
        """String operations through the linked runtime."""
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "main.tcl").write_text(
                """\
proc process {} {
    set s "  Hello World  "
    set trimmed [string trim $s]
    string length $trimmed
}
"""
            )
            mod = wasm_link(Path(tmp) / "main.tcl")
            result = _run_linked_module(mod, "::process")
            assert result == 11  # "Hello World" is 11 chars

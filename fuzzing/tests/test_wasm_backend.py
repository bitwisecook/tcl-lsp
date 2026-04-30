"""Smoke tests for the differential-fuzzer WASM backend.

Verifies the WASM backend integrates with the harness — compiles +
runs a tiny script, classifies a Tcl-level error, and filters out
inputs that depend on WASM-stubbed commands.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from fuzzing.harness import run_differential
from fuzzing.wasm_backend import is_available, run_wasm, uses_stubbed_command

pytestmark = pytest.mark.slow


@pytest.fixture(scope="module", autouse=True)
def _require_wasm_runtime() -> None:
    """Skip the whole module if the Zig runtime isn't buildable here."""
    if not is_available():
        pytest.skip("WASM runtime not available")


class TestStubFilter:
    """The stub filter must pull the SILENT_STUB / TRAPPING_STUB names
    out of the parity baseline and detect them in fuzz inputs."""

    def test_switch_is_filtered(self) -> None:
        assert uses_stubbed_command("switch -- $x { a {puts a} default {} }")

    def test_plain_set_passes(self) -> None:
        assert not uses_stubbed_command("set x 42\nputs $x\n")

    def test_word_inside_string_does_not_trip(self) -> None:
        # ``switch`` only matters as a command; a literal string that
        # happens to contain the word should not be filtered.
        assert not uses_stubbed_command('puts "the switch is on"\n')


class TestRunWasm:
    """The WASM backend must produce harness-compatible RunResult shapes."""

    def test_simple_puts(self) -> None:
        r = run_wasm("puts hello\n")
        assert r.return_code == 0
        assert r.stdout == "hello\n"

    def test_arithmetic(self) -> None:
        r = run_wasm("set x 1\nset y 2\nputs [expr {$x + $y}]\n")
        assert r.return_code == 0
        assert r.stdout.strip() == "3"


class TestHarnessIntegration:
    """``run_differential(use_wasm=True)`` must wire the new backend in
    alongside ``vm`` / ``vm_opt`` and produce no spurious mismatches on
    a trivially-correct script."""

    def test_wasm_appears_in_results(self) -> None:
        r = run_differential(
            "set x 42\nputs hello\n",
            use_tclsh=False,
            use_wasm=True,
        )
        assert "wasm" in r.results
        assert r.results["wasm"].return_code == 0

    def test_clean_script_no_mismatch(self) -> None:
        r = run_differential(
            "set x 42\nputs $x\n",
            use_tclsh=False,
            use_wasm=True,
        )
        assert r.mismatches == []

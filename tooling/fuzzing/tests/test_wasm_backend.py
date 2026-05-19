"""Smoke tests for the differential-fuzzer WASM backend.

Verifies the WASM backend integrates with the harness — compiles +
runs a tiny script, classifies a Tcl-level error, and filters out
inputs that depend on WASM-stubbed commands.

The stub-filter tests are pure regex over the parity baseline JSON
and run independently of wasmtime / the Zig runtime — they must
still execute in CI environments without those installed, since
that's where stub-filter regressions would otherwise slip through
unnoticed.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from tooling.fuzzing.harness import run_differential
from tooling.fuzzing.wasm_backend import is_available, run_wasm, uses_stubbed_command

pytestmark = pytest.mark.slow

_REPO_ROOT = Path(__file__).resolve().parent.parent.parent


def _require_wasm_runtime() -> None:
    """Skip if wasmtime + the Zig runtime aren't available.

    ``is_available()`` lazily triggers ``runtime_wasm_path()`` which
    can shell out to ``zig build``; a missing zig toolchain or build
    failure raises rather than returns ``False``.  Catch and skip so
    runtime-build failures don't error the whole class — same posture
    as the missing-wasmtime case.
    """
    pytest.importorskip("wasmtime", reason="wasmtime not installed")
    try:
        available = is_available()
    except Exception as exc:  # noqa: BLE001
        pytest.skip(f"WASM runtime not available: {exc}")
    if not available:
        pytest.skip("WASM runtime not available")


class TestStubFilter:
    """The stub filter must pull the SILENT_STUB / TRAPPING_STUB names
    out of the parity baseline and detect them in fuzz inputs.

    The detector lives at command-start positions only — non-command
    uses (variable name, parameter, string literal, identifier
    fragment) must NOT be flagged.
    """

    # Hits

    def test_switch_at_top_level(self) -> None:
        assert uses_stubbed_command("switch -- $x { a {puts a} default {} }")

    def test_switch_inside_braced_body(self) -> None:
        # Regression for the lexer-doesn't-recurse bug: a stubbed
        # command nested inside an ``if`` / ``proc`` / ``namespace
        # eval`` body must still be detected.  The previous
        # implementation walked the top-level token stream only and
        # missed these.
        assert uses_stubbed_command("if {1} {\n    switch -- $x { a {puts a} default {} }\n}\n")

    def test_switch_after_semicolon(self) -> None:
        assert uses_stubbed_command("set y 1; switch -- $x { default {} }")

    def test_switch_inside_command_substitution(self) -> None:
        assert uses_stubbed_command("set r [switch -- $x { default 1 }]\n")

    # Non-hits — these are the false-positives that the prior
    # token-text-equality filter incorrectly flagged.

    def test_plain_set_passes(self) -> None:
        assert not uses_stubbed_command("set x 42\nputs $x\n")

    def test_switch_as_variable_name(self) -> None:
        # ``switch`` here is the *variable*, not the command.  Filter
        # must not drop this script.
        assert not uses_stubbed_command("set switch 1\nputs $switch\n")

    def test_switch_as_loop_variable(self) -> None:
        assert not uses_stubbed_command("foreach switch {a b c} { puts $switch }\n")

    # Note — ``proc p {switch} { … }`` (stub name as a parameter
    # inside a brace-delimited list) is intentionally NOT covered by
    # a non-hit assertion.  Without full parsing the regex can't
    # distinguish a parameter list from a body, so it conservatively
    # treats ``{switch …`` as "command at body start".  For a fuzzer
    # filter the bias is correct — over-skipping a rare shape is
    # cheaper than feeding stub commands into the WASM backend and
    # logging phantom findings.  The script generator never emits
    # stub names as parameters, so this case doesn't appear in
    # practice.

    def test_word_inside_string_does_not_trip(self) -> None:
        # ``switch`` inside a literal string must not be filtered.
        assert not uses_stubbed_command('puts "the switch is on"\n')

    def test_substring_of_identifier_does_not_trip(self) -> None:
        # ``my_switch_proc`` is one identifier; the ``\b`` boundary
        # in the pattern must keep this from matching.
        assert not uses_stubbed_command("my_switch_proc arg1\n")

    def test_stubbed_command_inside_nested_braced_bodies(self) -> None:
        # Regression for issue #265.  The earlier lexer-walk filter
        # treated each braced body as a single ESC token and missed
        # every command word inside — so scripts that exercised a
        # stubbed command (e.g. ``switch``) only inside an ``if`` body
        # or a ``namespace eval`` slipped past the corpus-load gate
        # and the WASM backend then trapped on the stub, polluting
        # other findings' categories.  This snippet replicates the
        # exact shapes from ``fuzzing/findings/seed_1774200003.tcl``
        # (the literal example called out in the issue).
        script = (
            "namespace eval util {\n"
            "    switch -- 65 {\n"
            "        default { puts ok }\n"
            "    }\n"
            "}\n"
            "if {1} {\n"
            "    switch -- true {\n"
            "        default { puts ok }\n"
            "    }\n"
            "}\n"
        )
        assert uses_stubbed_command(script)

        # If the literal seed file is still in the tree, also verify
        # against it — protects against the regression returning in a
        # shape the synthetic snippet didn't anticipate.
        seed = _REPO_ROOT / "fuzzing" / "findings" / "seed_1774200003.tcl"
        if seed.exists():
            assert uses_stubbed_command(seed.read_text(encoding="utf-8"))


class TestRunWasm:
    """The WASM backend must produce harness-compatible RunResult shapes."""

    @pytest.fixture(scope="class", autouse=True)
    def _runtime_required(self) -> None:
        _require_wasm_runtime()

    def test_simple_puts(self) -> None:
        r = run_wasm("puts hello\n")
        assert r.return_code == 0
        assert r.stdout == "hello\n"

    def test_arithmetic(self) -> None:
        r = run_wasm("set x 1\nset y 2\nputs [expr {$x + $y}]\n")
        assert r.return_code == 0
        assert r.stdout.strip() == "3"

    def test_clean_run_after_timeout_does_not_trap_spuriously(self) -> None:
        # Regression: ``set_epoch_deadline`` takes an absolute counter
        # value and the wasmtime engine is reused across the whole
        # fuzz campaign.  An earlier shape of this code armed the
        # deadline at a fixed ``1`` every time, so once any prior
        # watchdog had bumped the engine, every subsequent run
        # trapped immediately before its script started executing —
        # poisoning the rest of the campaign.  A clean script run
        # AFTER a forced timeout must still execute and produce its
        # expected stdout.
        timeout_run = run_wasm("while {1} { set x 1 }\n", timeout=0.1)
        assert timeout_run.error_message == "TIMEOUT"
        # If the deadline weren't tracked, this would also TIMEOUT
        # (or trap with an epoch error) instead of running.
        clean_run = run_wasm("puts ok\n", timeout=2.0)
        assert clean_run.return_code == 0
        assert clean_run.stdout == "ok\n"


class TestHarnessIntegration:
    """``run_differential(use_wasm=True)`` must wire the new backend in
    alongside ``vm`` / ``vm_opt`` and produce no spurious mismatches on
    a trivially-correct script."""

    @pytest.fixture(scope="class", autouse=True)
    def _runtime_required(self) -> None:
        _require_wasm_runtime()

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

"""Focused regression tests for the Tier-2 wasm-runtime / codegen fixes.

Each test pins one contract that previously truncated an entire
``tcl9-tcltest-wasm`` stem at the bundle's first trap.  See
``tests/baselines/tcl9-tcltest-wasm/README.md`` for the full
fix-and-ratchet workflow.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> tuple[str, str]:
    wasm = _compile_tcl(source)
    out = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
    stdout = out[1] if len(out) >= 2 else ""
    stderr = out[2] if len(out) >= 3 else ""
    return stdout, stderr


class TestSubstNonAsciiAfterDollar:
    """``$`` followed by a non-name byte must round-trip as a literal ``$``.

    parse-12.26 ``Tcl_ParseVarName [d2ffcca163] non-ascii``: with the bug,
    ``"$г"`` traps under ``can't read "": no such variable`` because
    the variable-name scanner consumed zero bytes after ``$`` and still
    fell through to ``var_resolve`` with an empty name TclObj.
    """

    def test_subst_dollar_non_ascii(self) -> None:
        # Route through ``subst`` so the runtime's ``subst_flagged_full``
        # path executes (the static codegen has its own ``$г`` lowering
        # that bypasses subst altogether).
        stdout, _ = _run('puts [subst "$\\u0433"]\n')
        assert stdout.strip() == "$г"

    def test_subst_dollar_dot_dollar(self) -> None:
        # ``$.`` and ``$$`` mirror parse-12.18: the dollar is literal
        # whenever the next byte does not start an identifier.
        stdout, _ = _run('puts [subst {$.$$}]\n')
        assert stdout.strip() == "$.$$"


class TestStringSubcommandUnderArity:
    """Static-codegen ``string <sub>`` calls must surface the runtime
    wrong-args error when the call is short of the registry-declared
    minimum, instead of zero-padding the missing slots and silently
    returning the empty string (error-1.3 / 1.6 / 1.7 / cmdAH-1.4 / 1.5).
    """

    def test_string_index_no_args(self) -> None:
        src = (
            "catch {string index} b\n"
            "puts $b\n"
            'puts [info exists ::errorInfo]\n'
        )
        stdout, _ = _run(src)
        lines = stdout.splitlines()
        assert lines[0] == 'wrong # args: should be "string index string charIndex"'
        # ``::errorInfo`` must be stamped by ``tcl_cmd_error`` even when
        # the failure originates from the static codegen path.
        assert lines[1] == "1"

    def test_string_index_stamps_errorinfo(self) -> None:
        # Static codegen path: ``string index`` with no operands must
        # raise the wrong-args error AND stamp ``::errorInfo``.  Before
        # the fix, the codegen called ``tcl_string_index(0, 0)``, which
        # silently returned the empty string with no ``::errorInfo``
        # update — error-1.3 / 1.6 / 1.7 then could not match the
        # expected error trace.
        src = (
            "catch {string index} b\n"
            "puts $b\n"
            "puts [info exists ::errorInfo]\n"
            "puts $::errorInfo\n"
        )
        stdout, _ = _run(src)
        lines = stdout.splitlines()
        assert lines[0] == 'wrong # args: should be "string index string charIndex"'
        assert lines[1] == "1"
        assert lines[2].startswith('wrong # args: should be "string index')


class TestArrayElementInEvalFallback:
    """Bare ``$arr(idx)`` round-tripped through the eval-fallback must
    keep its array-element semantics — ``word_piece``'s previous span-vs-
    text check classified bare ``$arr($cmd)`` as braced and re-emitted it
    as ``${arr($cmd)}``, collapsing the recursive ``$cmd`` substitution
    into a literal scalar lookup of the source spelling
    (cmdAH-1.4 / 1.5 ``$numargErrors($cmd)``).
    """

    def test_global_array_substituted_key(self) -> None:
        src = (
            'set ::numargErrors(KEY) "RESULT"\n'
            "proc x {cmd} {\n"
            "    variable numargErrors\n"
            "    catch {undef-cmd $numargErrors($cmd)} r\n"
            "    puts $r\n"
            "}\n"
            "x KEY\n"
        )
        stdout, _ = _run(src)
        # Without the fix: ``can't read "numargErrors($cmd)": no such variable``.
        assert stdout.strip() == 'invalid command name "undef-cmd"'


class TestTcltestInternalsBootstrap:
    """The ``[namespace which -command …] eq ""`` guard around
    ``namespace eval ::tcltest::internals { … }`` (cmdIL bootstrap)
    must evaluate the empty-string compare correctly when the LHS is
    a ``[cmd]`` substitution.  ``eval_string_expr`` previously only
    stripped ``{…}`` quoting from the operand spans, so ``""`` survived
    as the two-byte string ``""`` and never compared equal to the empty
    LHS.
    """

    def test_double_quoted_empty_eq(self) -> None:
        stdout, _ = _run(
            'puts [expr {[namespace which -command ::nope] eq ""}]\n'
        )
        assert stdout.strip() == "1"

    def test_double_quoted_literal_eq(self) -> None:
        stdout, _ = _run(
            "set x abc\n"
            'puts [expr {$x eq "abc"}]\n'
        )
        assert stdout.strip() == "1"


class TestErrorRecursionLimit:
    """``eval_proc_call_bucket`` must raise reference Tcl's
    ``too many nested evaluations`` once nested eval depth crosses
    the configured ceiling, instead of riding wasmtime's call stack
    until ``call stack exhausted`` aborts the bundle (error-1.8 was
    the canonical reproducer in the wider tcltest slice).
    """

    def test_recursion_limit_bounded_iteration(self) -> None:
        # Drive the eval-fallback path enough times to push past the
        # ceiling.  Each iteration parks one frame, so ``parked_top``
        # is the actual measure of nesting depth here.
        src = (
            "proc up {n} {\n"
            "    if {$n <= 0} { return ok }\n"
            "    uplevel 1 [list up [expr {$n - 1}]]\n"
            "}\n"
            "set rc [catch {up 200} msg]\n"
            "puts $rc\n"
            "puts $msg\n"
        )
        stdout, _ = _run(src)
        lines = stdout.splitlines()
        # rc=1 (caught error); the message must be the recursion-limit
        # diagnostic, not whatever leaked through from the wasm trap.
        assert lines[0] == "1"
        assert "too many nested evaluations" in lines[1]

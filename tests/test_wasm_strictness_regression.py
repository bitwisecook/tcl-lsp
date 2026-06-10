"""WASM-runtime strictness regression tests.

End-to-end regressions for four strictness gaps the release audit found
GAPPED or only partially covered against the differential fuzzer.  Each
test compiles a tiny Tcl script, runs it under wasmtime, and asserts that
the runtime ERRORS with the exact C-Tcl wording rather than silently
computing a permissive result.

The error wording is captured into stdout via ``catch {...} msg`` +
``puts``, so we assert on the precise message the runtime produces
(mirroring the ``catch``/``puts $msg`` idiom in ``test_wasm_mathop.py``).

Issues covered:
  * #259 — ``if {1} {a} else {b} elseif {1} {c}`` (elseif after else)
    must raise ``wrong # args: extra words after "else" clause in "if"
    command``.
  * #261 — floating-point operand in a bitwise/shift op must raise
    ``cannot use floating-point value "<v>" as [left|right] operand of
    "<op>"``.
  * #262 — ``incr i`` where ``i`` holds a float string (``"52.60"``)
    must raise ``expected integer but got "52.60"``.  The AOT-compiled
    ``incr`` codegen path is still permissive here (silently advances to
    53), so the AOT case is an xfail tracking the live bug; the
    interpreted path is asserted to error.
  * #263 — reading an unset scalar inside ``expr`` (``expr {$count}``
    with ``count`` unset) must raise ``can't read "count": no such
    variable``.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from shared.runtime_wasm import runtime_wasm_path  # noqa: E402
from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

_ZIG_RUNTIME_PATH = runtime_wasm_path(build_if_missing=False)

pytestmark = pytest.mark.skipif(
    not _ZIG_RUNTIME_PATH.exists(),
    reason=f"Zig WASM runtime not built: {_ZIG_RUNTIME_PATH}",
)


def _run(source: str) -> str:
    """Compile + run ``source`` under wasmtime, returning stripped stdout."""
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.strip()


def _catch_msg(body: str) -> str:
    """Run ``body`` guarded by ``catch`` and return the captured ``[code msg]``.

    A passing strictness test produces ``1 {<error message>}``; a
    permissive (buggy) runtime produces ``0 {...}`` because the body ran
    without error.
    """
    return _run(f"puts [list [catch {{{body}}} msg] $msg]")


# ---------------------------------------------------------------------------
# #259 — elseif after else clause must error.
# ---------------------------------------------------------------------------


def test_elseif_after_else_clause_errors() -> None:
    """``if {1} {a} else {b} elseif {1} {c}`` must raise the extra-words
    error rather than silently running the if/else and discarding the
    trailing ``elseif`` clause.  Regression for issue #259."""
    out = _catch_msg("if {1} {set x 1} else {set x 2} elseif {1} {set x 3}")
    assert out == '1 {wrong # args: extra words after "else" clause in "if" command}'


# ---------------------------------------------------------------------------
# #261 — floating-point operand in bitwise / shift ops must error.
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "body,expected",
    [
        ("expr {~ -38.6}", 'cannot use floating-point value "-38.6" as operand of "~"'),
        (
            "expr {5 & 70.34}",
            'cannot use floating-point value "70.34" as right operand of "&"',
        ),
        (
            "expr {-23.55 << 1}",
            'cannot use floating-point value "-23.55" as left operand of "<<"',
        ),
    ],
)
def test_float_operand_in_bitwise_or_shift_errors(body: str, expected: str) -> None:
    """A floating-point operand in a bitwise (``~`` ``&``) or shift
    (``<<``) operator must error with the C-Tcl 9 wording rather than
    truncating the float and computing a result.  Regression for #261."""
    assert _catch_msg(body) == f"1 {{{expected}}}"


# ---------------------------------------------------------------------------
# #262 — float string where an integer is required (incr) must error.
# ---------------------------------------------------------------------------


def _run_interp(source: str) -> str:
    """Run ``source`` through the runtime interpreter (eval-fallback path).

    Mirrors the helper in ``test_wasm_incr_errorinfo.py`` — wrapping the
    body in ``set s {...}; eval $s`` forces the runtime interpreter
    (rather than the AOT-compiled statement codegen) to execute it.
    """
    wrapped = "set __s {" + source + "}\neval $__s\n"
    wasm = _compile_tcl(wrapped)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.strip()


def test_incr_float_string_errors_interpreted() -> None:
    """``incr i`` with ``i = "52.60"`` must raise ``expected integer but
    got "52.60"`` — a float string is rejected by Tcl's strict-integer
    parser.  Asserted on the runtime-interpreter path (the AOT path is
    covered, as xfail, by :func:`test_incr_float_string_errors_aot`).
    Regression for issue #262."""
    out = _run_interp("set i 52.60\nputs [list [catch {incr i} msg] $msg]")
    assert out == '1 {expected integer but got "52.60"}'


@pytest.mark.xfail(
    strict=True,
    reason=(
        "Issue #262: the inline `_emit_incr` codegen fallback (used in "
        "un-optimised positions such as a `catch {}` body) is permissive — "
        '`catch {incr i}` with i="52.60" returns `0 53` (and i="abc" '
        "returns `0 1`) instead of erroring. Root cause: the fallback reads "
        "the operand via `_emit_unbox_int` -> the runtime `obj_get_int` "
        "(runtime/zig/valtypes/tcl_obj.zig), which truncates a float/garbage "
        "string rather than raising; the optimised top-level path routes "
        "through the strict `tcl_incr` helper instead, which is why a bare "
        "top-level `incr` traps correctly. Fix: make `_emit_incr` enforce the "
        "strict-integer guard (call `tcl_incr` / a strict accessor) on the "
        "inline path too, then remove this xfail. Needs WASM-baseline "
        "re-validation, so it is tracked here rather than fixed inline."
    ),
)
def test_incr_float_string_errors_aot() -> None:
    """AOT-compiled counterpart of the interpreted #262 test.

    Routes ``incr i`` through the inline statement/catch-body codegen
    rather than the runtime interpreter or the optimised top-level path.
    This is the live permissiveness bug: the float string ``"52.60"``
    must error, but the inline path returns 53.  Marked
    ``xfail(strict=True)`` so it flips to a hard failure (forcing the
    marker's removal) the moment the codegen path is fixed.
    """
    out = _run("set i 52.60\nputs [list [catch {incr i} msg] $msg]")
    assert out == '1 {expected integer but got "52.60"}'


# ---------------------------------------------------------------------------
# #263 — reading an unset scalar inside expr must error.
# ---------------------------------------------------------------------------


def test_unset_scalar_in_expr_errors() -> None:
    """``expr {$count}`` with ``count`` never set must raise ``can't read
    "count": no such variable`` rather than substituting 0/empty and
    rolling forward.  Regression for issue #263 (the scalar-in-expr case
    the audit found uncovered)."""
    assert _catch_msg("expr {$count}") == '1 {can\'t read "count": no such variable}'


def test_unset_scalar_in_expr_arithmetic_errors() -> None:
    """The seed-shape repro: ``set y [expr {$count + 1}]`` must error on
    the unset read before any arithmetic.  Regression for issue #263."""
    out = _catch_msg("set y [expr {$count + 1}]")
    assert out == '1 {can\'t read "count": no such variable}'


def test_unset_scalar_in_expr_inside_proc_errors() -> None:
    """The in-proc seed shape: an unset local read inside ``expr`` must
    error rather than defaulting.  Regression for issue #263."""
    out = _run("proc p {} { return [expr {$undefined * 2}] }\nputs [list [catch {p} msg] $msg]")
    assert out == '1 {can\'t read "undefined": no such variable}'

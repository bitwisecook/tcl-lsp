# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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


def test_incr_float_string_errors_aot() -> None:
    """AOT-compiled counterpart of the interpreted #262 test.

    Routes ``incr i`` through the inline statement/catch-body codegen
    (the last statement of a ``catch {}`` body) rather than the runtime
    interpreter or the optimised top-level path.  This was the live
    permissiveness bug: the inline ``_emit_incr`` fallback read the
    operand via the permissive ``obj_get_int`` and silently returned 53.
    It now routes through the strict ``tcl_incr`` helper (the scan layer
    requests the import for inline/value/body ``incr`` too), so a
    non-integer current value errors like every other path.
    """
    out = _run("set i 52.60\nputs [list [catch {incr i} msg] $msg]")
    assert out == '1 {expected integer but got "52.60"}'


def test_incr_garbage_string_errors_aot() -> None:
    """Companion to the float case: ``incr`` of a non-numeric string in a
    ``catch`` body must error rather than treating the value as 0 and
    returning 1.  Regression for issue #262."""
    out = _run("set i abc\nputs [list [catch {incr i} msg] $msg]")
    assert out == '1 {expected integer but got "abc"}'


def test_incr_value_position_float_string_errors() -> None:
    """``[incr i]`` in value position (command substitution) with a float
    string must also enforce the strict-integer guard.  Regression for
    issue #262 (the value-context inline path)."""
    assert _catch_msg("set i 52.60; incr i") == '1 {expected integer but got "52.60"}'


def test_incr_nonint_amount_errors() -> None:
    """A non-integer *increment* (``incr i 2.5``) must error too — the
    strict guard applies to both operands.  Regression for issue #262."""
    assert _catch_msg("set i 5; incr i 2.5") == '1 {expected integer but got "2.5"}'


def test_incr_normal_paths_still_work() -> None:
    """Positive controls: integer ``incr`` in value/catch/statement
    position still increments, and ``incr`` on an unset scalar still
    initialises to 0 (Tcl 8.5+)."""
    assert _run("set i 5\nputs [incr i]") == "6"
    assert _catch_msg("set i 5; incr i") == "0 6"
    assert _run("proc p {} {incr x}\nputs [p]") == "1"


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

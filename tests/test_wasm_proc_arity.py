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

"""WASM proc call-time arity — too-many-arguments rejection.

A proc whose last parameter is not the variadic ``args`` collector
rejects extra arguments with ``wrong # args: should be "<name> ...>"``
(proc-old-30.3).  ``eval_proc_call_bucket`` in
``runtime/zig/interp/tcl_interp.zig`` previously only checked the
too-FEW case (a required parameter with no supplied value), so extra
arguments were silently dropped.  Mirrors ``ProcWrongNumArgs`` in
tclProc.c.

The slice runs test bodies INTERPRETED (``eval_script``), so the procs
below are defined and called through that path via
``set s {…}; eval $s``.

The check is enforced before the compiled-proc dispatch in
``eval_proc_call_bucket`` (so it covers compiled procs too), and the
static-codegen call sites route the over-arity case through the eval
fallback — so the ``_run_compiled`` cases below (ordinary non-``eval``
source, which compiles the call to a direct proc call) raise the same
error rather than silently truncating (Codex review on PR #532).
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run_interp(source: str) -> str:
    wrapped = "set __s {" + source + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(wrapped), capture_stdout=True)
    return stdout.rstrip("\n")


def _run_compiled(source: str) -> str:
    """Compile *source* directly — the proc call becomes a direct
    static-codegen call (not eval-dispatched)."""
    _, stdout = _run_wasm(_compile_tcl(source), capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # Exact arg count — fine.
        ("proc t {x y z} {list $x $y $z}\nputs [t 1 2 3]", "1 2 3"),
        # Too many — error with the full parameter signature (proc-old-30.3).
        (
            'proc t {x y z} {list $x $y $z}\nputs "[catch {t 1 2 3 4} m]:$m"',
            '1:wrong # args: should be "t x y z"',
        ),
        # ``args`` collector accepts any number of trailing arguments.
        ("proc t {a args} {list $a $args}\nputs [t 1 2 3 4]", "1 {2 3 4}"),
        # Zero-parameter proc rejects any argument.
        ("proc t {} {return ok}\nputs [t]", "ok"),
        (
            'proc t {} {return ok}\nputs "[catch {t x} m]:$m"',
            '1:wrong # args: should be "t"',
        ),
        # Defaulted trailing parameter still rejects a true excess and
        # renders as ``?y?`` in the signature.
        ("proc t {x {y 9}} {list $x $y}\nputs [t 1 2]", "1 2"),
        (
            'proc t {x {y 9}} {list $x $y}\nputs "[catch {t 1 2 3} m]:$m"',
            '1:wrong # args: should be "t x ?y?"',
        ),
        # Too FEW still errors (unchanged regression guard).
        (
            'proc t {x y z} {list $x $y $z}\nputs "[catch {t 1 2} m]:$m"',
            '1:wrong # args: should be "t x y z"',
        ),
    ],
)
def test_proc_call_arity(source: str, expected: str) -> None:
    assert _run_interp(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # The Codex-flagged shape: ordinary (non-eval) source compiles
        # ``t 1 2 3 4`` to a direct proc call.  Must raise, not truncate.
        (
            'proc t {x y z} {list $x $y $z}\nputs "[catch {t 1 2 3 4} m]:$m"',
            '1:wrong # args: should be "t x y z"',
        ),
        # value context ([proc ...] substitution)
        (
            'proc t {x y z} {list $x $y $z}\nset r [catch {t 1 2 3 4} m]\nputs "$r:$m"',
            '1:wrong # args: should be "t x y z"',
        ),
        # expression context (if {[proc ...]})
        (
            'proc t {x y z} {return 1}\nif {[catch {t 1 2 3 4} m]} {puts "e:$m"} else {puts ok}',
            'e:wrong # args: should be "t x y z"',
        ),
        # Exact / variadic / defaulted compiled calls are unaffected.
        ("proc t {x y z} {list $x $y $z}\nputs [t 1 2 3]", "1 2 3"),
        ("proc t {a args} {list $a $args}\nputs [t 1 2 3 4]", "1 {2 3 4}"),
        ("proc t {x {y 9}} {list $x $y}\nputs [t 1]", "1 9"),
        (
            'proc t {x {y 9}} {list $x $y}\nputs "[catch {t 1 2 3} m]:$m"',
            '1:wrong # args: should be "t x ?y?"',
        ),
    ],
)
def test_proc_call_arity_compiled(source: str, expected: str) -> None:
    assert _run_compiled(source) == expected

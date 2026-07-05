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

"""Rename-aware compiled dispatch for proc calls in command substitutions.

A dynamic ``rename $x $y`` distrusts every builtin in the unit, and the
command-substitution emitters route distrusted command names to the
interpreter eval-fallback.  For a deeply self-recursive ``expr`` body the
eval-fallback pushes ~10 wasm frames per level and overflows the
wasm-stack recursion gate within ~20 levels — the mechanism behind
compExpr-old-3.8 (the obfuscated "twelve days" proc, ~445 levels deep:
computes 2358 compiled, but used to overflow once any unrelated dynamic
``rename`` distrusted the builtins).

``_emit_distrust_proc_subst`` instead emits a guarded direct call: it
verifies at run time that the name still maps to *this exact* compiled
proc (``proc_get_func_idx(proc_lookup(name)) == func_idx``) and that
tracing is quiescent (``exec_trace_quiescent``), then takes the cheap
direct ``call`` (one wasm frame per level).  Any rename / redefinition or
active trace falls back to the interpreter.  The whole guard is disabled
for units that use command / execution traces (see
``wasm_codegen_module``), which never carry this recursion shape.

See ``_emit_distrust_proc_subst`` in
``compiler/codegen/wasm/_emitter/_commands.py``.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

_DISTRUST = "set q junk; catch {rename $q other}\n"


def _run(source: str) -> str:
    wasm = _compile_tcl(_DISTRUST + source)
    _, stdout = _run_wasm(wasm, capture_stdout=True, timeout_s=60)
    return stdout.rstrip("\n")


def test_deep_expr_recursion_in_subst_survives_distrust() -> None:
    # Recursive call inside the expr body's command substitution, 300 deep
    # (well past the ~60-level interpreter-dispatch wasm-stack gate).
    src = "proc g {n} {expr {$n<=0?0:[g [expr {$n-1}]]}}\nputs [g 300]\n"
    assert _run(src) == "0"


def test_deep_recursion_as_command_argument_survives_distrust() -> None:
    # Recursive call as a command *argument* (value-context substitution).
    src = (
        "proc descend {n s} {expr {$n<=0?[string length $s]:"
        "[descend [expr {$n-1}] [string range $s 1 end]]}}\n"
        "puts [descend 300 abc]\n"
    )
    assert _run(src) == "0"


def test_digit_leading_proc_name_recurses_under_distrust() -> None:
    # ``12days``-style digit-leading name resolves as a compiled proc.
    src = "proc 12d {n} {expr {$n<=0?0:[12d [expr {$n-1}]]}}\nputs [12d 300]\n"
    assert _run(src) == "0"


def test_guard_falls_back_when_proc_renamed() -> None:
    # If the proc is renamed away, the guard's runtime func_idx check
    # fails and the interpreter reports the correct error.
    src = "proc g {n} {expr {$n}}\nrename g h\nputs [list [catch {expr {[g 1]}} m] $m]\n"
    assert _run(src) == '1 {invalid command name "g"}'


def test_redefined_proc_dispatches_latest_body_under_distrust() -> None:
    # A runtime redefinition must dispatch the new body, not the stale
    # compiled one (the func_idx guard distinguishes them).
    src = "proc f {} {return 1}\nproc caller {} {expr {[f]}}\nproc f {} {return 2}\nputs [caller]\n"
    assert _run(src) == "2"

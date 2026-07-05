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

"""WASM ``incr`` — the ``(reading increment)`` errorInfo frame.

C Tcl adds ``\\n    (reading increment)`` to ``::errorInfo`` ONLY when the
explicit *increment argument* fails to parse as an integer (``incr x 1a``
— incr-2.30..2.33, incr-old-2.5).  When the *variable's own value* is the
non-integer (``incr x`` with ``x`` set to ``abc`` — incr-old-2.4) there is
no increment being read, so the frame is omitted and the enclosing log
stays ``while executing`` rather than ``invoked from within``.

``runtime/zig/interp/tcl_ns.zig::raise_expected_integer`` previously
appended the frame unconditionally.  The slice runs test bodies
INTERPRETED (``eval_script``), so these force that path via
``set s {…}; eval $s``.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run_interp(source: str) -> str:
    wrapped = "set __s {" + source + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(wrapped), capture_stdout=True)
    return stdout.rstrip("\n")


def test_incr_bad_variable_value_no_reading_increment_frame() -> None:
    # incr-old-2.4: the variable value is the non-integer.
    src = "set x abc\nputs [list [catch {incr x} msg] $msg $::errorInfo]\n"
    expected = (
        '1 {expected integer but got "abc"} {expected integer but got "abc"\n'
        '    while executing\n"incr x"}'
    )
    assert _run_interp(src) == expected


def test_incr_bad_increment_arg_has_reading_increment_frame() -> None:
    # incr-old-2.5: the explicit increment argument is the non-integer.
    src = "set x 123\nputs [list [catch {incr x 1a} msg] $msg $::errorInfo]\n"
    expected = (
        '1 {expected integer but got "1a"} {expected integer but got "1a"\n'
        "    (reading increment)\n"
        '    invoked from within\n"incr x 1a"}'
    )
    assert _run_interp(src) == expected

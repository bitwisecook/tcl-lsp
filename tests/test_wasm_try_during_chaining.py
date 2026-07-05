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

"""WASM ``dict exists`` nested paths + ``try`` ``-during`` exception chaining.

Pins the two root-cause fixes behind the error-18.8/.9/.10 cluster of the
``tcl9-tcltest-wasm`` slice:

* ``dict exists DICT KEY ?KEY ...?`` must walk the *whole* key chain, not
  just the first key.  The runtime handler (``runtime/zig/cmds/dict.zig``)
  and both WASM codegen dispatch paths
  (``compiler/codegen/wasm/_emitter/cmds/dict_.py``) previously consulted
  only ``words[3]``, so ``dict exists $opts -during -during`` (and any
  multi-key probe) returned true whenever the first key existed.  Matches
  ``tclDictObj.c::DictExistsCmd``: a missing key OR a non-dict intermediate
  collapses to false without raising.  error-18.8 relied on this.

* ``try ... on/trap ... finally`` exception chaining (the ``-during``
  options key).  Mirrors C Tcl's ``TryPostHandler`` / ``TryPostFinal``
  (``tclCmdMZ.c``):

    - a handler that raises a *direct* error (``throw`` / ``error``) chains
      the body's options under ``-during`` (error-16.8/.9, -18.10);
    - a handler that re-raises via ``return -options`` reproduces the
      body's options verbatim with NO ``-during`` (error-15.10);
    - a handler that completes cleanly replaces the body's options
      wholesale (error-16.10, -18.9);
    - a finally that raises chains the *current* (handler-or-body) options
      under ``-during`` (error-18.7, -18.8, -18.9, -18.10).

  The slice runs test bodies INTERPRETED (``eval_script``), so the ``try``
  cases below force that path via ``set s {…}; eval $s``.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run_compiled(source: str) -> str:
    """Compile *source* directly (static codegen path) and return stdout."""
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


def _run_interp(source: str) -> str:
    """Run *source* through the interpreter (``eval``), as the slice does."""
    wrapped = "set __s {" + source + "}\neval $__s\n"
    wasm = _compile_tcl(wrapped)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


# ---------------------------------------------------------------------------
# dict exists — nested key paths
# ---------------------------------------------------------------------------

_DICT_EXISTS_CASES = [
    # Single key (unchanged behaviour) — present / absent.
    ("puts [dict exists {a 1 b 2} a]", "1"),
    ("puts [dict exists {a 1 b 2} z]", "0"),
    # Two-level descent into a nested dict.
    ("puts [dict exists {a {b c}} a b]", "1"),
    ("puts [dict exists {a {b c}} a x]", "0"),
    # First key present but second absent — the bug returned 1 here.
    ("puts [dict exists {-code 1 -during {-code 0 -level 0}} -during -during]", "0"),
    ("puts [dict exists {-code 1 -during {-code 0 -level 0}} -during -code]", "1"),
    ("puts [dict exists {-code 1 -during {-code 0 -level 0}} -during -bogus]", "0"),
    # Non-dict intermediate value collapses to false, never raises.
    ("puts [dict exists {a b} a b c]", "0"),
    # Three-level descent.
    ("puts [dict exists {a {b {c d}}} a b c]", "1"),
    ("puts [dict exists {a {b {c d}}} a b x]", "0"),
]


@pytest.mark.parametrize("source,expected", _DICT_EXISTS_CASES)
def test_dict_exists_nested_interpreted(source: str, expected: str) -> None:
    assert _run_interp(source) == expected


@pytest.mark.parametrize("source,expected", _DICT_EXISTS_CASES)
def test_dict_exists_nested_compiled(source: str, expected: str) -> None:
    # The static codegen path must route the multi-key form through the
    # eval fallback (the 2-param runtime import only walks one key).
    assert _run_compiled(source) == expected


def test_dict_exists_nested_compiled_in_proc() -> None:
    """Multi-key ``dict exists`` inside a compiled proc body."""
    src = (
        "proc f {} {\n"
        "  set d {a {b c} d e}\n"
        '  return "[dict exists $d a b] [dict exists $d a x] [dict exists $d d x]"\n'
        "}\n"
        "puts [f]\n"
    )
    assert _run_compiled(src) == "1 0 0"


def test_dict_exists_nested_compiled_in_condition() -> None:
    """Multi-key ``dict exists`` in an ``if`` condition (expr context)."""
    src = (
        "set d {a {b c}}\n"
        "if {[dict exists $d a b]} { puts yes-ab } else { puts no-ab }\n"
        "if {[dict exists $d a x]} { puts yes-ax } else { puts no-ax }\n"
    )
    assert _run_compiled(src) == "yes-ab\nno-ax"


def test_dict_get_nested_compiled_in_condition() -> None:
    """Multi-key ``dict get`` in an ``expr`` comparison."""
    src = "set d {a {b 7}}\nif {[dict get $d a b] == 7} { puts seven } else { puts other }\n"
    assert _run_compiled(src) == "seven"


# ---------------------------------------------------------------------------
# try ... finally — ``-during`` exception chaining (error-18.x)
# ---------------------------------------------------------------------------


def test_during_body_ok_handler_ok_finally_error() -> None:
    """error-18.8: body OK, ``on ok`` handler OK, finally throws.

    ``-during`` is the handler's clean options (``-code 0``) with no
    nested ``-during``.
    """
    src = (
        "catch { try { list a b c } on ok {} { list d e f }"
        " finally { throw BAR baz } } em opts\n"
        "puts [list [dict get $opts -during -code]"
        " [dict exists $opts -during -during]]\n"
    )
    assert _run_interp(src) == "0 0"


def test_during_body_error_handler_ok_finally_error() -> None:
    """error-18.9: body error, ``on error`` handler OK, finally throws.

    The clean handler replaces the body's error options, so ``-during``
    is ``-code 0`` — not the body's ``-code 1``.
    """
    src = (
        "catch { try { throw FOO bar } on error {} { list d e f }"
        " finally { throw BAR baz } } em opts\n"
        "puts [list [dict get $opts -during -code]"
        " [dict exists $opts -during -during]]\n"
    )
    assert _run_interp(src) == "0 0"


def test_during_body_error_handler_error_finally_error() -> None:
    """error-18.10: body error, handler throws, finally throws.

    Two-level chain: the finally's ``-during`` is the handler's BAR error,
    whose own ``-during`` is the body's FOO error.
    """
    src = (
        "catch { try { throw FOO bar } on error {} { throw BAR baz }"
        " finally { throw BAR baz } } em opts\n"
        "puts [list [dict get $opts -during -errorcode]"
        " [dict get $opts -during -during -errorcode]]\n"
    )
    assert _run_interp(src) == "BAR FOO"


def test_during_finally_only_body_error() -> None:
    """error-18.7: body error, finally throws — ``-during`` is the body."""
    src = (
        "catch { try { throw FOO bar } finally { throw BAR baz } } em opts\n"
        "puts [dict get $opts -during -errorcode]\n"
    )
    assert _run_interp(src) == "FOO"


def test_during_handler_error_no_finally_chains_body() -> None:
    """error-16.8/.9: handler error without finally still chains ``-during``.

    Verifies the chaining is computed deterministically (it previously
    only "passed" in the bundle via leaked cross-test ``during_options``).
    """
    # body OK, ``on ok`` handler throws -> -during is the body's OK options.
    src_ok = (
        "catch { try { list a b c } on ok {em opts} { throw BAR baz } }"
        " tryem tryopts\n"
        "puts [list [dict get $tryopts -during -code]"
        " [dict exists $tryopts -during -during]]\n"
    )
    assert _run_interp(src_ok) == "0 0"
    # body error, handler throws -> -during is the body's FOO error.
    src_err = (
        "catch { try { throw FOO bar } trap {} {em opts} { throw BAR baz } }"
        " tryem tryopts\n"
        "puts [dict get $tryopts -during -errorcode]\n"
    )
    assert _run_interp(src_err) == "FOO"


def test_during_handler_ok_no_chain() -> None:
    """error-16.10: a clean handler leaves no ``-during`` at all."""
    src = (
        "catch { try { throw FOO bar } trap {} {em opts} { list d e f } }"
        " tryem tryopts\n"
        "puts [dict exists $tryopts -during]\n"
    )
    assert _run_interp(src) == "0"


def test_return_options_reraise_no_during() -> None:
    """error-15.10: a ``return -options`` re-raise reproduces the body's
    options verbatim, with no spurious ``-during``."""
    src = (
        "set script {return -level 1 -code 1 foo}\n"
        "catch $script m1 o1\n"
        "catch {try $script on 1 {x y} {return -options $y $x}} m2 o2\n"
        "puts [list [dict exists $o2 -during]"
        " [expr {[lsort -stride 2 $o1] eq [lsort -stride 2 $o2]}]]\n"
    )
    assert _run_interp(src) == "0 1"

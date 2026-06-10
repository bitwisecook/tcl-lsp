"""WASM ``unset arr(elem)`` — interpreted array-element removal.

The interpreted ``unset`` handler (``runtime/zig/cmds/var.zig``)
mishandled the ``arr(key)`` form: it ran ``array_unset`` against the
literal name ``arr(key)`` (a no-op) and then ``var_set(arr(key), 0)``,
which routed through the array path and merely *set* the element to the
empty string — leaving the element present and the array directory
inconsistent.  It now splits ``arr(key)``, resolves the array name
through any ``global`` / ``upvar`` link, and calls
``array_unset_element``, matching ``array unset arr key``.  Regression
guard for proc-old-3.3 / 3.5 and the set-old element-unset cases.

The slice runs test bodies INTERPRETED (``eval_script``), so these force
that path via ``set s {…}; eval $s``.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run_interp(source: str) -> str:
    wrapped = "set __s {" + source + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(wrapped), capture_stdout=True)
    return stdout.rstrip("\n")


def test_unset_element_toplevel() -> None:
    src = (
        "set a(x) 1\nset a(y) 2\nunset a(y)\n"
        'puts "[lsort [array names a]] [info exists a(y)] [info exists a(x)]"'
    )
    assert _run_interp(src) == "x 0 1"


def test_unset_element_through_global_link() -> None:
    # proc-old-3.3 / 3.5: ``global a; unset a(y)`` removes the element
    # from the global array, leaving the other element intact.
    src = (
        "set b(p) 1\nset b(q) 2\n"
        "proc tp {} {global b; unset b(q)}\ntp\n"
        'puts "[lsort [array names b]] [info exists b(q)]"'
    )
    assert _run_interp(src) == "p 0"


def test_unset_missing_element_errors() -> None:
    src = 'set a(x) 1\nputs "[catch {unset a(z)} m]:$m"'
    assert _run_interp(src) == '1:can\'t unset "a(z)": no such element in array'


def test_unset_nocomplain_missing_element() -> None:
    src = 'set a(x) 1\nputs "[catch {unset -nocomplain a(z)} m]:$m [array names a]"'
    assert _run_interp(src) == "0: x"


def test_unset_whole_array_still_works() -> None:
    src = 'set a(x) 1\nset a(y) 2\nunset a\nputs "[info exists a] [array exists a]"'
    assert _run_interp(src) == "0 0"

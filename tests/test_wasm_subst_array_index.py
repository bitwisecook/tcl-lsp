"""WASM ``subst`` — error propagation from an array-element index.

``$arr(idx)`` substitution recursively substitutes the index span.  When
that recursion raised an error — e.g. an unterminated ``${`` in
``$arr(${foo)`` (``missing close-brace for variable name``) — the outer
``subst`` ignored it and ran the array lookup with an empty key,
clobbering the real diagnostic with ``can't read "arr()": no such
variable`` on the static path, or trapping when the doubled error state
unwound through the interpreter.  ``subst_flagged_full``
(``runtime/zig/parse/tcl_subst.zig``) now propagates the index error
verbatim.  Regression guard for parse-18.11 / 18.12.

The index input is built with ``format`` (``{`` = 123, ``$`` = 36) so the
test source itself stays free of unbalanced braces under the
``eval``-wrapped interpreted path.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

# ``foo$array(${foo)`` — an unterminated ``${`` inside an array index.
_BUILD_INPUT = (
    'set d [format %c 36]\nset ob [format %c 123]\nset inp "foo${d}array(${d}${ob}foo)"\n'
)
_EXPECT = "1 {missing close-brace for variable name}"


def _run_static(source: str) -> str:
    _, stdout = _run_wasm(_compile_tcl(source), capture_stdout=True)
    return stdout.rstrip("\n")


def _run_interp(source: str) -> str:
    wrapped = "set __s {" + source + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(wrapped), capture_stdout=True)
    return stdout.rstrip("\n")


def test_subst_unterminated_brace_in_index_static() -> None:
    src = _BUILD_INPUT + "puts [list [catch {subst $inp} msg] $msg]"
    assert _run_static(src) == _EXPECT


def test_subst_unterminated_brace_in_index_interp() -> None:
    src = _BUILD_INPUT + "puts [list [catch {subst $inp} msg] $msg]"
    assert _run_interp(src) == _EXPECT


def test_subst_unterminated_brace_bare_dollar_paren() -> None:
    # parse-18.12 shape: ``foo$(${foo)`` — ``$(`` is a literal ``$``, and
    # the trailing ``${foo`` is the unterminated brace.
    src = (
        "set d [format %c 36]\n"
        "set ob [format %c 123]\n"
        'set inp "foo${d}(${d}${ob}foo)"\n'
        "puts [list [catch {subst $inp} msg] $msg]"
    )
    assert _run_interp(src) == _EXPECT


def test_subst_valid_array_index_still_works() -> None:
    # The error path must not disturb a well-formed ``$arr(idx)`` subst.
    src = 'set arr(k) hello\nset d [format %c 36]\nset inp "${d}arr(k)"\nputs [subst $inp]'
    assert _run_interp(src) == "hello"

"""Runtime tests for the ``lset`` command.

Covers the shapes the emitter handles directly:

* flat index (``lset v i newval``)
* ``end`` / ``end-N`` indices
* multi-dim flat indices (``lset v i j k newval``)
* indices-as-list (``lset v {i j} newval``)

Each test compiles a minimal Tcl snippet, runs it under wasmtime,
and asserts the stdout printed by a final ``puts`` matches the
expected result.  Regression for PR #180 follow-up — the runtime
now carries a real ``tcl_cmd_list_set`` instead of trapping at
dispatch.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm


def _run(src: str) -> str:
    wasm, _ = _compile_tcl_with_diag(src)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout


class TestLsetFlatIndex:
    def test_single_index(self) -> None:
        out = _run("set lst {a b c d e}\nlset lst 2 X\nputs $lst\n")
        assert out == "a b X d e\n"

    def test_first_element(self) -> None:
        out = _run("set lst {a b c}\nlset lst 0 Z\nputs $lst\n")
        assert out == "Z b c\n"

    def test_last_element_via_end(self) -> None:
        out = _run("set lst {a b c d}\nlset lst end X\nputs $lst\n")
        assert out == "a b c X\n"

    def test_end_minus_n(self) -> None:
        out = _run("set lst {a b c d e}\nlset lst end-1 X\nputs $lst\n")
        assert out == "a b c X e\n"


class TestLsetQuoting:
    def test_value_with_space_brace_wraps(self) -> None:
        # A value with whitespace must be brace-wrapped so the result
        # reparses as the same number of elements.
        out = _run("set lst {a b c}\nlset lst 1 {two three}\nputs $lst\n")
        assert out == "a {two three} c\n"

    def test_nested_list_preserves_grouping(self) -> None:
        # Non-target elements that were braced should keep their
        # braces so nested lists don't flatten.
        out = _run("set lst {{a b} {c d} {e f}}\nlset lst 1 Z\nputs $lst\n")
        assert out == "{a b} Z {e f}\n"


class TestLsetMultiDim:
    def test_flat_multi_index(self) -> None:
        # ``lset v i j newval`` — nested access via multiple args.
        out = _run("set lst {{1 2} {3 4}}\nlset lst 1 1 X\nputs $lst\n")
        assert out == "{1 2} {3 X}\n"

    def test_index_list(self) -> None:
        # ``lset v {i j} newval`` — nested access via index list.
        out = _run("set lst {{1 2} {3 4}}\nlset lst {1 1} X\nputs $lst\n")
        assert out == "{1 2} {3 X}\n"

    def test_three_deep(self) -> None:
        out = _run("set lst {{{1 2} {3 4}} {{5 6} {7 8}}}\nlset lst 1 0 1 Z\nputs $lst\n")
        assert out == "{{1 2} {3 4}} {{5 Z} {7 8}}\n"


class TestLsetReturnsNewValue:
    def test_return_value_is_new_list(self) -> None:
        # ``lset`` returns the new value of the variable — that's the
        # updated list as a whole, not just the replaced element.
        out = _run(
            "proc p {} {\n"
            "    set lst {a b c}\n"
            "    set ret [lset lst 1 X]\n"
            "    return $ret\n"
            "}\n"
            "puts [p]\n"
        )
        assert out == "a X c\n"

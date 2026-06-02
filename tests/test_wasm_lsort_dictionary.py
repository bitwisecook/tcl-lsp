"""WASM ``lsort -dictionary`` comparison semantics.

Pins ``lsort_dict_cmp`` in ``runtime/zig/cmds/list.zig`` against C
``DictionaryCompare`` (``tclCmdIL.c``):

* the primary comparison is case-INSENSITIVE (characters are folded to
  lower case), so punctuation between ``Z`` and ``a`` sorts before ``A``;
* a case difference only breaks an otherwise-equal tie, and then
  upper-case sorts before lower-case;
* a leading-zero count is likewise a secondary tiebreak (more leading
  zeros sorts later), recorded only at the first difference so an
  earlier case tiebreak still wins.

Regression guard for cmdIL-4.17 / 4.20 / 4.28 .. 4.33 / 4.3.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # Case is a tiebreak, not primary — cmdIL-4.17 / 4.20.
        ("puts [lsort -dictionary {aBCd abcc}]", "abcc aBCd"),
        ("puts [lsort -dictionary {abcd ABce}]", "abcd ABce"),
        # Punctuation between Z and a sorts before A (fold to lower) —
        # cmdIL-4.28 / 4.31 / 4.32.
        ("puts [lsort -dictionary [list AA ` c CC]]", "` AA c CC"),
        ("puts [lsort -dictionary [list AA c ` CC]]", "` AA c CC"),
        # A broader punctuation spread — cmdIL-4.33.
        ("puts [lsort -dictionary [list AA ! c CC `]]", "! ` AA c CC"),
        ("puts [lsort -dictionary [list AA ^ c CC `]]", "^ ` AA c CC"),
        # Case tiebreak outranks the leading-zero tiebreak — cmdIL-4.3.
        ("puts [lsort -dictionary {a3b A03b}]", "A03b a3b"),
        # Embedded numbers still compare numerically (unchanged).
        ("puts [lsort -dictionary {x10 x9 x1}]", "x1 x9 x10"),
        ("puts [lsort -dictionary {1k 0k 10k}]", "0k 1k 10k"),
    ],
)
def test_lsort_dictionary(source: str, expected: str) -> None:
    assert _run(source) == expected

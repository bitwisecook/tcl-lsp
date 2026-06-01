"""WASM ``switch`` with patterns supplied as separate (substitutable) words.

When the arms come as separate words rather than a single braced
``{pat body ...}`` block, each pattern is an ordinary word that undergoes
``$var`` / ``[cmd]`` substitution before matching.  The codegen used to
emit every switch pattern as a literal, so ``switch -glob -- $s $pat
{...}`` matched against the literal text ``$pat`` and never fired —
breaking the ``Wrapper_Tcl_StringMatch`` helper that backs util-5.*:

    proc W {pattern string} {
        switch -glob -- $string $pattern {return 1} default {return 0}
    }

The ``IRSwitch.patterns_braced`` flag now distinguishes the two forms so
separate-word patterns are emitted via ``_emit_value``.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

_WRAPPER = (
    "proc W {pattern string} {\n"
    "    switch -glob -- $string $pattern {return 1} default {return 0}\n"
    "}\n"
)


def _run(body: str) -> str:
    wasm = _compile_tcl(_WRAPPER + body)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "call,expected",
    [
        ("puts [W ab*c abc]", "1"),
        ("puts [W a?c abc]", "1"),
        ("puts [W *c abc]", "1"),
        ("puts [W *3*6*9 0123456789]", "1"),
        ("puts [W xyz abc]", "0"),
        ("puts [W {[abc]bc} abc]", "1"),
    ],
)
def test_switch_glob_dynamic_pattern(call: str, expected: str) -> None:
    assert _run(call) == expected


def test_switch_exact_dynamic_pattern() -> None:
    # Exact mode with a variable pattern must also substitute.
    body = "set p hello\nputs [switch -- hello $p {set r yes} default {set r no}]\n"
    assert _run(body) == "yes"


def test_switch_pattern_only_import_is_scanned() -> None:
    # A separate-word pattern is emitted via ``_emit_value`` and may be the
    # *only* token needing a runtime helper (e.g. ``$arr(key)`` -> tcl_array_get).
    # The import pre-scan must scan patterns when they aren't braced, or the
    # module instantiates without the helper and the pattern miscompiles
    # (PR #516 review).
    body = (
        "proc P {} { set a(p) {hel*}; "
        "switch -glob -- hello $a(p) {return MATCH} default {return NO} }\n"
        "puts [P]\n"
    )
    assert _run(body) == "MATCH"

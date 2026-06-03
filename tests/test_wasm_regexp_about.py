"""WASM ``regexp -about`` — compiled-RE introspection.

Pins ``regexp_about`` in ``runtime/zig/valtypes/tcl_regex.zig``, mirroring
C Tcl's ``TclRegAbout``: ``regexp -about exp`` compiles *exp* (consuming
only the pattern operand, no subject) and returns the two-element list
``{re_nsub {flag-name…}}``.  ``re_nsub`` is the capture-group count; the
flags are the ``REG_U*`` bits the Spencer engine recorded in ``re_info``
(empty ``{}`` for a plain RE, a bare word for one flag, brace-wrapped for
two or more).  The option was previously an unimplemented stub that
raised ``wrong # args`` or trapped (regexpComp-20.2).
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
        # regexpComp-20.2: a plain RE has no subexpressions and no flags.
        ("puts [list [catch {regexp -about abc} m] $m]", "0 {0 {}}"),
        ("puts [regexp -about abc]", "0 {}"),
        # Capture groups are counted.
        ("puts [regexp -about {(a)(b)c}]", "2 {}"),
        ("puts [regexp -about {(a(b(c)))}]", "3 {}"),
        # Anchors add no flags.
        ("puts [regexp -about {^a$}]", "0 {}"),
        # A single non-POSIX feature reports one bare flag word.
        ("puts [regexp -about {(?i)abc}]", "0 REG_UNONPOSIX"),
        # Two or more flags are returned as a brace-wrapped sublist.
        ("puts [regexp -about {(a)\\1}]", "1 {REG_UBACKREF REG_UNONPOSIX}"),
        ("puts [regexp -about {a(?=b)}]", "0 {REG_ULOOKAHEAD REG_UNONPOSIX}"),
        # ``-about`` honours preceding option flags (e.g. -nocase compiles
        # but adds no REG_U bits for a plain pattern).
        ("puts [regexp -nocase -about abc]", "0 {}"),
    ],
)
def test_regexp_about(source: str, expected: str) -> None:
    assert _run(source) == expected


def test_regexp_about_requires_pattern() -> None:
    # ``regexp -about`` with no pattern is a wrong-# args error.
    out = _run("puts [catch {regexp -about} m]")
    assert out == "1"

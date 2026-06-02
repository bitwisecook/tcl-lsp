"""WASM ``namespace which -variable`` for declared-but-unset variables.

``variable foo`` with no value declares ``foo`` in the namespace as an
existing-but-unset variable; ``namespace which -variable foo`` must still
report its fully-qualified name.  The runtime stores such a declaration
as a ``value = 0`` entry in the flat namespace var table, which the
value-gated existence probe missed — so ``namespace which`` returned the
empty string (var-5.2).  ``eval_namespace_which`` now also accepts a bare
declaration via ``variable_declared_in_flat_globals``.

See ``eval_namespace_which`` in ``runtime/zig/cmds/tcl_cmd_interp.zig``.

The ``eval { variable v }`` wrapper routes the declaration through the
interpreted command path (the runtime fix); a bare compile-time
``variable v`` with no value is handled by a separate codegen path.
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
        # var-5.2: a bare ``variable`` declaration resolves to its FQN.
        (
            "namespace eval test_ns_var {eval {variable martha}; "
            "puts [namespace which -variable martha]}",
            "::test_ns_var::martha",
        ),
        # A variable that later gains a value still resolves.
        (
            "namespace eval t {eval {variable v}; set v 5; puts [namespace which -variable v]}",
            "::t::v",
        ),
        # A never-declared variable still returns the empty string.
        (
            "namespace eval t {eval {variable v}; puts <[namespace which -variable nope]>}",
            "<>",
        ),
        # Command resolution is unaffected.
        (
            "namespace eval t {proc foo {} {}; puts [namespace which -command foo]}",
            "::t::foo",
        ),
    ],
)
def test_namespace_which_variable(source: str, expected: str) -> None:
    assert _run(source) == expected

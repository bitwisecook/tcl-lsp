"""WASM ``apply`` — lambda parameter specs with defaults and validation.

``eval_apply`` treated each whole parameter element as the variable
name, so a ``{name default}`` spec became a variable literally named
``"name default"`` and default values were never applied.  It now parses
each parameter as a ``{name ?default?}`` list:

* element 0 is the variable name, optional element 1 the default used
  when no argument is supplied (apply-8.*);
* a ``{}`` spec (no name) raises ``argument with no name`` (apply-2.2);
* a ``{a b c}`` spec (>2 fields) raises ``too many fields in argument
  specifier "a b c"`` (apply-2.3).
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(body: str) -> str:
    _, stdout = _run_wasm(_compile_tcl(body), capture_stdout=True)
    return stdout.rstrip("\n")


def test_apply_defaults_all_omitted() -> None:
    body = "set L [list {{x 1} {y 2}} {list $x $y}]\nputs [apply $L]\n"
    assert _run(body) == "1 2"


def test_apply_default_partial() -> None:
    body = "set L [list {x {y 2}} {list $x $y}]\nputs [apply $L 5]\n"
    assert _run(body) == "5 2"


def test_apply_default_overridden() -> None:
    body = "set L [list {{x 1} {y 2}} {list $x $y}]\nputs [apply $L 7 9]\n"
    assert _run(body) == "7 9"


def test_apply_malformed_no_name() -> None:
    body = "set L [list {{}} boo]\nputs [catch {apply $L} msg]:$msg\n"
    assert _run(body) == "1:argument with no name"


def test_apply_malformed_too_many_fields() -> None:
    body = "set L [list {{a b c}} boo]\nputs [catch {apply $L} msg]:$msg\n"
    assert _run(body) == '1:too many fields in argument specifier "a b c"'

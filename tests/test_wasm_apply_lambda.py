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


# ---------------------------------------------------------------------------
# (parsing lambda expression ...) errorInfo frame + formal validation +
# info level 0.  The slice runs test bodies INTERPRETED (eval_script), so
# these force that path via ``set s {…}; eval $s`` to match it.  Mirrors
# C Tcl's SetLambdaFromAny / TclCreateProc (apply-2.2..2.5, 6.2, 6.3).
# ---------------------------------------------------------------------------


def _run_interp(source: str) -> str:
    wrapped = "set __s {" + source + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(wrapped), capture_stdout=True)
    return stdout.rstrip("\n")


def test_apply_2_2_errorinfo_frame() -> None:
    src = "set lambda [list {{}} boo]\nputs [list [catch {apply $lambda} msg] $msg $::errorInfo]\n"
    expected = (
        "1 {argument with no name} {argument with no name\n"
        '    (parsing lambda expression "{{}} boo")\n'
        '    invoked from within\n"apply $lambda"}'
    )
    assert _run_interp(src) == expected


def test_apply_2_3_errorinfo_frame() -> None:
    src = (
        "set lambda [list {{a b c}} boo]\n"
        "puts [list [catch {apply $lambda} msg] $msg $::errorInfo]\n"
    )
    expected = (
        '1 {too many fields in argument specifier "a b c"} '
        '{too many fields in argument specifier "a b c"\n'
        '    (parsing lambda expression "{{a b c}} boo")\n'
        '    invoked from within\n"apply $lambda"}'
    )
    assert _run_interp(src) == expected


def test_apply_2_4_array_element_formal() -> None:
    src = "set lambda [list a(1) boo]\nputs [list [catch {apply $lambda} msg] $msg $::errorInfo]\n"
    expected = (
        '1 {formal parameter "a(1)" is an array element} '
        '{formal parameter "a(1)" is an array element\n'
        '    (parsing lambda expression "a(1) boo")\n'
        '    invoked from within\n"apply $lambda"}'
    )
    assert _run_interp(src) == expected


def test_apply_2_5_qualified_formal() -> None:
    src = "set lambda [list a::b boo]\nputs [list [catch {apply $lambda} msg] $msg $::errorInfo]\n"
    expected = (
        '1 {formal parameter "a::b" is not a simple name} '
        '{formal parameter "a::b" is not a simple name\n'
        '    (parsing lambda expression "a::b boo")\n'
        '    invoked from within\n"apply $lambda"}'
    )
    assert _run_interp(src) == expected


def test_apply_open_paren_not_array_element() -> None:
    # A scalar formal containing ``(`` but no trailing ``)`` is valid.
    assert _run_interp("puts [apply {{a(b} {return ok}} hi]") == "ok"


def test_apply_6_2_info_level_no_args() -> None:
    src = "set lambda [list {} {info level 0}]\nputs [apply $lambda]\n"
    assert _run_interp(src) == "apply {{} {info level 0}}"


def test_apply_6_3_info_level_with_args() -> None:
    src = "set lambda [list args {info level 0}]\nputs [apply $lambda x y]\n"
    assert _run_interp(src) == "apply {args {info level 0}} x y"

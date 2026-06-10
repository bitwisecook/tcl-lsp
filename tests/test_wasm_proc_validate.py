"""WASM ``proc`` definition-time argument validation.

Pins the validation added to ``runtime/zig/cmds/proc.zig``'s
``eval_proc`` handler, mirroring C Tcl's ``TclCreateProc``:

* arity — ``proc`` requires exactly ``name args body`` (proc-old-5.1 /
  5.2 / 5.3);
* the argument spec must be a well-formed list (proc-old-5.4);
* each formal is ``{name ?default?}`` — an empty spec is ``argument
  with no name`` (proc-old-5.5 / 5.6) and a >2-field spec is ``too many
  fields in argument specifier`` (proc-old-5.7);
* a malformed spec leaves the command undefined (proc-old-5.11).

The forms below route through the runtime handler (rather than the
compiler's static proc registration) by building the command from a
variable / ``eval``, which is what tcltest's dynamic test bodies do.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


_WRONG_ARGS = 'wrong # args: should be "proc name args body"'


@pytest.mark.parametrize(
    "source,expected",
    [
        # Arity — too few / too many positional arguments.
        (r"set c {proc}; puts [list [catch {eval $c} m] $m]", f"1 {{{_WRONG_ARGS}}}"),
        (
            r"set c {proc tproc b}; puts [list [catch {eval $c} m] $m]",
            f"1 {{{_WRONG_ARGS}}}",
        ),
        (
            r"set c {proc tproc b c d e}; puts [list [catch {eval $c} m] $m]",
            f"1 {{{_WRONG_ARGS}}}",
        ),
        # Malformed argument-spec list — unmatched brace.
        (
            'set s "\\{xyz"; puts [list [catch {proc tproc $s {return foo}} m] $m]',
            "1 {unmatched open brace in list}",
        ),
        # Empty formal — no name.
        (
            r"set s {{} y}; puts [list [catch {proc tproc $s {return foo}} m] $m]",
            "1 {argument with no name}",
        ),
        # >2 fields in a formal.
        (
            r"set s {{x 1 2} y}; puts [list [catch {proc tproc $s {return foo}} m] $m]",
            '1 {too many fields in argument specifier "x 1 2"}',
        ),
        # A failed definition must leave the command undefined.
        (
            r"set s {x {} z}; catch {proc tq $s {return foo}}; puts [list [catch {tq 1} m] $m]",
            '1 {invalid command name "tq"}',
        ),
        # A valid definition still works.
        (r"proc good {a b} {expr {$a + $b}}; puts [good 2 3]", "5"),
        # A formal with a default is fine (2 fields).
        (
            r"set s {a {b 7}}; proc okp $s {expr {$a + $b}}; puts [okp 3]",
            "10",
        ),
    ],
)
def test_proc_arg_validation(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # A command word that evaluates to the empty string is a genuine
        # invocation of a command literally named "" — NOT a no-op — so a
        # proc defined with an empty name is reachable (proc-3.7).  These
        # cases wrap the body in ``eval`` to drive the interpreted
        # ``eval_command_dispatch`` path; the static codegen path is
        # covered separately in ``test_proc_empty_name_static_codegen``.
        (r"set s {proc {} {} {return ok-empty}; puts [{}]}; eval $s", "ok-empty"),
        (
            r"set s {proc {} {x} {return got-$x}; puts [{} 42]}; eval $s",
            "got-42",
        ),
        # Calling the empty-named proc with the wrong arity reports the
        # name list-element-quoted as ``{}`` (Tcl_WrongNumArgs runs every
        # word through TclScanElement / TclConvertElement), not a bare
        # leading space (proc-3.7).
        (
            r"set s {proc {} {x} {}; puts [catch {{}} m]:$m}; eval $s",
            '1:wrong # args: should be "{} x"',
        ),
        # An undefined empty command name falls through to the canonical
        # ``invalid command name ""`` rather than silently succeeding
        # (proc-old-9.1: ``set v [t1]; catch {$v}`` where t1 returns "").
        (
            r'set s {puts [catch {$e} m]:$m}; set e ""; eval $s',
            '1:invalid command name ""',
        ),
        # The quoting generalises: a proc whose name contains whitespace
        # is wrapped in braces in the wrong-args message too.
        (
            r"set s {proc {a b} {x y} {}; puts [catch {{a b} 1} m]:$m}; eval $s",
            '1:wrong # args: should be "{a b} x y"',
        ),
    ],
)
def test_proc_empty_and_quoted_command_name(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # Ordinary (non-``eval``) source compiles the ``proc {}`` body
        # statically and registers it via ``proc_register_compiled``.
        # That path previously rejected an empty simple name, leaving the
        # command undefined so a later ``{}`` call trapped ``invalid
        # command name ""``.  It now registers (matching the interpreted
        # ``proc_register``) and the compiled body is reachable (proc-3.7
        # through the static codegen path, not just the eval-fallback).
        ("proc {} {} {return ok-empty}\nputs [{}]", "ok-empty"),
        ("proc {} {x} {return got-$x}\nputs [{} 7]", "got-7"),
        # Redefining the empty-named command swaps the body in place.
        (
            "proc {} {} {return v1}\nputs [{}]\nproc {} {} {return v2}\nputs [{}]",
            "v1\nv2",
        ),
    ],
)
def test_proc_empty_name_static_codegen(source: str, expected: str) -> None:
    assert _run(source) == expected
